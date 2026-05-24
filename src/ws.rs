use crate::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::{Message, Session};
use dashmap::DashMap;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::task;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserSessionInfo {
    pub server_no: String,
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerDeliveryEvent {
    pub session_id: String,
    pub msg: String,
}


#[derive(Clone)]
pub struct WsManager {
    pub server_no: String,
    // High-performance concurrent hashmap to hold active local sessions
    pub local_sessions: Arc<DashMap<String, Session>>,
    redis: web::Data<RedisManager>,
    pool: web::Data<DbPool>,
}

impl WsManager {
    pub async fn new(
        server_no: String,
        redis: web::Data<RedisManager>,
        pool: web::Data<DbPool>,
    ) -> Self {
        let local_sessions = Arc::new(DashMap::new());

        let manager = Self {
            server_no: server_no.clone(),
            local_sessions: local_sessions.clone(),
            redis: redis.clone(),
            pool,
        };

        // Bootstrapping the PubSub listener for cross-server message routing
        let redis_clone = manager.redis.clone();
        let sessions_clone = manager.local_sessions.clone();
        let server_channel = format!("server_msgs:{}", server_no);

        tokio::spawn(async move {
            let _ = redis_clone.subscribe_json::<ServerDeliveryEvent, _>(&server_channel, move |event| {
                // If the targeted session is on this server, send the message instantly
                if let Some(mut session_ref) = sessions_clone.get_mut(&event.session_id) {
                    let mut session = session_ref.clone();
                    let msg = event.msg.clone();
                    
                // actix_ws::Session ops are async, so we spawn a lightweight task
                    tokio::spawn(async move {
                        let _ = session.text(msg).await;
                    });
                }
            }).await;
        });

        manager
    }
}

pub async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    user_id_path: web::Path<String>,
    ws_manager: web::Data<WsManager>,
) -> Result<HttpResponse, Error> {
    let user_id = user_id_path.into_inner();
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;
    let session_id = Uuid::new_v4().to_string();

    // 1. Add session to localized concurrency map
    ws_manager.local_sessions.insert(session_id.clone(), session.clone());

    // 2. Publish connectivity to Global Redis Hashmap
    let info = UserSessionInfo {
        server_no: ws_manager.server_no.clone(),
        session_id: session_id.clone(),
    };
    let info_json = serde_json::to_string(&info).unwrap();
    let _ = ws_manager.redis.hset("user_sessions", &user_id, &info_json).await;

    // 3. Fetch and deliver unseen DB messages
    let pool = &ws_manager.pool;
    if let Ok(Some(user_row)) = pool.query_opt("SELECT unseen FROM users WHERE id = $1", &[&user_id]).await {
        let unseen: serde_json::Value = user_row.get("unseen");
        if !unseen.is_null() && unseen != serde_json::json!({}) {
            let _ = session.text(unseen.to_string()).await;
            // Clear unseen object after successful dispatch
            let _ = pool.execute("UPDATE users SET unseen = '{}'::jsonb WHERE id = $1", &[&user_id]).await;
        }
    }

    let manager_clone = ws_manager.clone();
    let uid_clone = user_id.clone();
    let sid_clone = session_id.clone();

    // 4. Stream watcher (Handles Incoming Msgs & Disconnections)
    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Ping(bytes) => {
                    let _ = session.pong(&bytes).await;
                }
                Message::Text(text) => {
                    // Logic to process direct messages from the client to the server
                }
                Message::Close(_) => break,
                _ => {}
            }
        }

        // --- DISCONNECT / CLEANUP ROUTINE ---
        manager_clone.local_sessions.remove(&sid_clone);
        
        // Ensure we only clear Redis if the user hasn't immediately reconnected elsewhere
        if let Ok(Some(current_info_str)) = manager_clone.redis.hget("user_sessions", &uid_clone).await {
            if let Ok(current_info) = serde_json::from_str::<UserSessionInfo>(&current_info_str) {
                if current_info.session_id == sid_clone {
                    // Assuming RedisManager exposes standard hdel or raw command execution
                    let _ = manager_clone.redis.hset("user_sessions", &uid_clone, "").await; // Overwrite/Delete
                }
            }
        }
    });

    Ok(response)
}


impl WsManager {
    /// Dispatches a message to an entire room. Safe to call from any HTTP route.
    pub async fn send_to_room(&self, room_no: &str, sender_id: &str, msg_content: &str) {
        let formatted_msg = format!("{}:{}", sender_id, msg_content);

        // 1. Log the message into the room's master history
        let _ = self.pool.execute(
            "UPDATE rooms SET msg = array_append(msg, $1) WHERE id = $2",
            &[&formatted_msg, &room_no]
        ).await;

        // 2. Scan all users registered to the room
        if let Ok(Some(room_row)) = self.pool.query_opt("SELECT users FROM rooms WHERE id = $1", &[&room_no]).await {
            let room_users: serde_json::Value = room_row.get("users");
            
            if let Some(users_obj) = room_users.as_object() {
                for (uid, _) in users_obj {
                    let mut delivered = false;

                    // 3. Routing Check
                    if let Ok(Some(session_json)) = self.redis.hget("user_sessions", uid).await {
                        if let Ok(session_info) = serde_json::from_str::<UserSessionInfo>(&session_json) {
                            if session_info.server_no == self.server_no {
                                // Target user is physically on THIS server
                                if let Some(mut session) = self.local_sessions.get_mut(&session_info.session_id) {
                                    let _ = session.text(formatted_msg.clone()).await;
                                    delivered = true;
                                }
                            } else {
                                // Target user is on ANOTHER server instance -> Publish cross-server event
                                let event = ServerDeliveryEvent {
                                    session_id: session_info.session_id,
                                    msg: formatted_msg.clone(),
                                };
                                let channel = format!("server_msgs:{}", session_info.server_no);
                                let _ = self.redis.publish(&channel, event).await;
                                delivered = true;
                            }
                        }
                    }

                    // 4. Save to `unseen` payload if offline
                    if !delivered {

// High-performance Postgres JSONB array append
let _ = self.pool.execute(r#"UPDATE users 
SET unseen = jsonb_set(
COALESCE(unseen, '{}'::jsonb), ARRAY[$1], COALESCE(unseen->$1, '[]'::jsonb) || $2::jsonb) WHERE id = $3"#, &[&room_no, &serde_json::to_value(vec![formatted_msg.clone()]).unwrap(), &uid]).await;
                    }
                }
            }
        }
    }

    /// Fetches the raw master record of the room, as requested
    pub async fn get_room_msg(&self, room_no: &str) -> Option<serde_json::Value> {
        let row = self.pool.query_opt("SELECT msg, users FROM rooms WHERE id = $1", &[&room_no]).await.unwrap();
        row.map(|r| {
            serde_json::json!({
                "msg": r.get::<Vec<String>, _>("msg"),
                "users": r.get::<serde_json::Value, _>("users")
            })
        })
    }
}
