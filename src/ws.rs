use actix_web::{web, Error, HttpRequest, HttpResponse, get};
use actix_ws::Message;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc::Sender; // Changed to Bounded Sender
use tracing::{error, info};
use uuid::Uuid;

// Assuming these are defined in your crate
use crate::prelude::{RedisManager, DbPool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub id: Uuid,
    pub msg_type: String, 
    pub room_id: Option<String>,
    pub sender_id: String,
    pub recipient_id: Option<String>,
    pub content: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ClientAction {
    JoinRoom { room_id: String },
    LeaveRoom { room_id: String },
    SendRoom { room_id: String, content: serde_json::Value },
    SendUser { user_id: String, content: serde_json::Value },
    SendUsers { user_ids: Vec<String>, content: serde_json::Value },
    Ping,
}

/// Internal events pushed to a connection's channel.
/// We use Arc<String> for messages to prevent CPU overhead from repeated JSON serialization.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    Message(Arc<String>),
    Error(String),
    Pong,
    Disconnect,
}

pub struct Connection {
    pub user_id: String,
    pub sender: Sender<ServerEvent>,
    pub rooms: DashMap<String, ()>,
}

#[derive(Default)]
pub struct ConnectionManager {
    pub connections: DashMap<String, Connection>,
    pub rooms: DashMap<String, DashMap<String, ()>>,
}

impl ConnectionManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            connections: DashMap::new(),
            rooms: DashMap::new(),
        })
    }

    pub fn add_connection(&self, user_id: String, sender: Sender<ServerEvent>) {
        self.connections.insert(user_id.clone(), Connection {
            user_id,
            sender,
            rooms: DashMap::new(),
        });
    }

    pub fn remove_connection(&self, user_id: &str) -> Option<Connection> {
        let conn = self.connections.remove(user_id)?.1;
        
        // Anti-Deadlock: Collect room IDs first without holding locks on the user's DashMap
        let room_ids: Vec<String> = conn.rooms.iter().map(|r| r.key().clone()).collect();
        
        for rid in room_ids {
            if let Some(members) = self.rooms.get(&rid) {
                members.remove(user_id);
                if members.is_empty() {
                    drop(members); // Drop the lock explicitly before mutating the parent map
                    self.rooms.remove(&rid);
                }
            }
        }
        Some(conn)
    }

    pub fn join_room(&self, user_id: &str, room_id: &str) {
        self.rooms.entry(room_id.to_string()).or_default().insert(user_id.to_string(), ());
        if let Some(conn) = self.connections.get(user_id) {
            conn.rooms.insert(room_id.to_string(), ());
        }
    }

    pub fn leave_room(&self, user_id: &str, room_id: &str) {
        if let Some(conn) = self.connections.get(user_id) {
            conn.rooms.remove(room_id);
        }
        if let Some(members) = self.rooms.get(room_id) {
            members.remove(user_id);
            if members.is_empty() {
                drop(members);
                self.rooms.remove(room_id);
            }
        }
    }

    pub fn get_room_online_users(&self, room_id: &str) -> Vec<String> {
        self.rooms
            .get(room_id)
            .map(|m| m.iter().map(|e| e.key().clone()).collect())
            .unwrap_or_default()
    }

    pub fn is_online(&self, user_id: &str) -> bool {
        self.connections.contains_key(user_id)
    }

    pub fn send_to_user(&self, user_id: &str, event: ServerEvent) -> bool {
        if let Some(conn) = self.connections.get(user_id) {
            // try_send safely manages backpressure. 
            // If the client channel is full, they are treated as offline.
            conn.sender.try_send(event).is_ok()
        } else {
            false
        }
    }
}

#[derive(Clone)]
pub struct WsService {
    redis: RedisManager,
    db: DbPool,
    manager: Arc<ConnectionManager>,
}

impl WsService {
    pub fn new(redis: RedisManager, db: DbPool) -> Self {
        Self {
            redis,
            db,
            manager: ConnectionManager::new(),
        }
    }

    pub async fn connect(&self, user_id: String, sender: Sender<ServerEvent>) -> anyhow::Result<()> {
        self.manager.add_connection(user_id.clone(), sender.clone());

        let online_key = format!("ws:user:{}:online", user_id);
        self.redis.set(&online_key, &true).await?;
        self.redis.expire(&online_key, 3600).await?;

        self.deliver_unseen(&user_id, None, &sender).await?;

        let user_rooms_key = format!("ws:user:{}:rooms", user_id);
        let rooms: Vec<String> = self.redis.smembers(&user_rooms_key).await.unwrap_or_default();
        for room_id in rooms {
            self.deliver_unseen(&user_id, Some(&room_id), &sender).await?;
        }

        info!("User {} connected", user_id);
        Ok(())
    }

    pub async fn disconnect(&self, user_id: &str) -> anyhow::Result<()> {
        let conn = self.manager.remove_connection(user_id);
        let online_key = format!("ws:user:{}:online", user_id);
        self.redis.del(&online_key).await.ok();

        // OFFLOAD DATABASE OPERATIONS: We spawn a background thread so the socket can close instantly 
        // without waiting on slow PostgreSQL N+1 insertions.
        let redis = self.redis.clone();
        let db = self.db.clone();
        let uid = user_id.to_string();
        let mut rooms = Vec::new();
        
        if let Some(c) = conn {
            rooms = c.rooms.iter().map(|r| r.key().clone()).collect();
        }

        tokio::spawn(async move {
            let unseen_key = format!("ws:user:{}:unseen", uid);
            let unseen: Vec<WsMessage> = redis.lrange(&unseen_key, 0, -1).await.unwrap_or_default();
            if !unseen.is_empty() {
                for msg in &unseen {
                    let sql = "INSERT INTO ws_unseen (id, user_id, room_id, sender_id, content, created_at)
                               VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING";
                    let room_null: Option<String> = None;
                    let _ = db.execute(sql, &[&msg.id, &uid, &room_null, &msg.sender_id, &msg.content, &msg.timestamp]).await;
                }
                redis.del(&unseen_key).await.ok();
            }

            for room_id in rooms {
                let r_unseen_key = format!("ws:room:{}:user:{}:unseen", room_id, uid);
                let unseen: Vec<WsMessage> = redis.lrange(&r_unseen_key, 0, -1).await.unwrap_or_default();
                if !unseen.is_empty() {
                    for msg in &unseen {
                        let sql = "INSERT INTO ws_unseen (id, user_id, room_id, sender_id, content, created_at)
                                   VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING";
                        let _ = db.execute(sql, &[&msg.id, &uid, &room_id, &msg.sender_id, &msg.content, &msg.timestamp]).await;
                    }
                    redis.del(&r_unseen_key).await.ok();
                }
            }
        });

        info!("User {} disconnected. DB sync offloaded.", user_id);
        Ok(())
    }

    pub async fn join_room(&self, user_id: &str, room_id: &str) -> anyhow::Result<()> {
        self.redis.sadd(&format!("ws:room:{}:members", room_id), user_id).await?;
        self.redis.sadd(&format!("ws:user:{}:rooms", user_id), room_id).await?;
        self.manager.join_room(user_id, room_id);

        if let Some(conn) = self.manager.connections.get(user_id) {
            self.deliver_unseen(user_id, Some(room_id), &conn.sender).await?;
        }

        let msg = WsMessage {
            id: Uuid::new_v4(),
            msg_type: "system".into(),
            room_id: Some(room_id.into()),
            sender_id: "system".into(),
            recipient_id: None,
            content: serde_json::json!({"event": "user_joined", "user_id": user_id}),
            timestamp: Utc::now(),
        };
        self.send_to_room(room_id, msg).await?;
        Ok(())
    }

    pub async fn leave_room(&self, user_id: &str, room_id: &str) -> anyhow::Result<()> {
        self.redis.srem(&format!("ws:room:{}:members", room_id), user_id).await?;
        self.redis.srem(&format!("ws:user:{}:rooms", user_id), room_id).await?;
        self.manager.leave_room(user_id, room_id);

        let msg = WsMessage {
            id: Uuid::new_v4(),
            msg_type: "system".into(),
            room_id: Some(room_id.into()),
            sender_id: "system".into(),
            recipient_id: None,
            content: serde_json::json!({"event": "user_left", "user_id": user_id}),
            timestamp: Utc::now(),
        };
        self.send_to_room(room_id, msg).await?;
        Ok(())
    }

    pub async fn send_to_room(&self, room_id: &str, msg: WsMessage) -> anyhow::Result<()> {
        let hist_key = format!("ws:room:{}:history", room_id);
        self.redis.rpush(&hist_key, &msg).await?;
        self.redis.expire(&hist_key, 86400).await.ok();

        let online = self.manager.get_room_online_users(room_id);
        let online_set: HashSet<String> = online.iter().cloned().collect();

        // Serialize string ONE TIME and wrap in Arc to save CPU
        let serialized_msg = Arc::new(serde_json::to_string(&msg)?);

        for uid in &online {
            if uid != &msg.sender_id {
                self.manager.send_to_user(uid, ServerEvent::Message(Arc::clone(&serialized_msg)));
            }
        }

        let all_members: Vec<String> = self.redis
            .smembers(&format!("ws:room:{}:members", room_id))
            .await
            .unwrap_or_default();

        for uid in &all_members {
            if !online_set.contains(uid) && uid != &msg.sender_id {
                let unseen_key = format!("ws:room:{}:user:{}:unseen", room_id, uid);
                self.redis.rpush(&unseen_key, &msg).await?;
                self.redis.expire(&unseen_key, 7 * 86400).await.ok();
            }
        }
        Ok(())
    }

    pub async fn send_to_user(&self, user_id: &str, msg: WsMessage) -> anyhow::Result<()> {
        let serialized_msg = Arc::new(serde_json::to_string(&msg)?);
        if self.manager.send_to_user(user_id, ServerEvent::Message(serialized_msg)) {
            return Ok(());
        }

        let unseen_key = format!("ws:user:{}:unseen", user_id);
        self.redis.rpush(&unseen_key, &msg).await?;
        self.redis.expire(&unseen_key, 7 * 86400).await.ok();
        Ok(())
    }

    pub async fn send_to_users(&self, user_ids: &[String], msg: WsMessage) -> anyhow::Result<()> {
        let mut offline = Vec::with_capacity(user_ids.len());
        let serialized_msg = Arc::new(serde_json::to_string(&msg)?);

        for uid in user_ids {
            if !self.manager.send_to_user(uid, ServerEvent::Message(Arc::clone(&serialized_msg))) {
                offline.push(uid.clone());
            }
        }

        for uid in &offline {
            let key = format!("ws:user:{}:unseen", uid);
            self.redis.rpush(&key, &msg).await?;
            self.redis.expire(&key, 7 * 86400).await.ok();
        }
        Ok(())
    }

    pub async fn handle_action(&self, user_id: &str, action: ClientAction) -> anyhow::Result<()> {
        match action {
            ClientAction::JoinRoom { room_id } => self.join_room(user_id, &room_id).await,
            ClientAction::LeaveRoom { room_id } => self.leave_room(user_id, &room_id).await,
            ClientAction::SendRoom { room_id, content } => {
                let msg = WsMessage {
                    id: Uuid::new_v4(),
                    msg_type: "text".into(),
                    room_id: Some(room_id.clone()),
                    sender_id: user_id.into(),
                    recipient_id: None,
                    content,
                    timestamp: Utc::now(),
                };
                self.send_to_room(&room_id, msg).await
            }
            ClientAction::SendUser { user_id: target, content } => {
                let msg = WsMessage {
                    id: Uuid::new_v4(),
                    msg_type: "direct".into(),
                    room_id: None,
                    sender_id: user_id.into(),
                    recipient_id: Some(target.clone()),
                    content,
                    timestamp: Utc::now(),
                };
                self.send_to_user(&target, msg).await
            }
            ClientAction::SendUsers { user_ids, content } => {
                let msg = WsMessage {
                    id: Uuid::new_v4(),
                    msg_type: "direct".into(),
                    room_id: None,
                    sender_id: user_id.into(),
                    recipient_id: None,
                    content,
                    timestamp: Utc::now(),
                };
                self.send_to_users(&user_ids, msg).await
            }
            ClientAction::Ping => {
                if let Some(conn) = self.manager.connections.get(user_id) {
                    let _ = conn.sender.try_send(ServerEvent::Pong);
                }
                Ok(())
            }
        }
    }

    async fn deliver_unseen(&self, user_id: &str, room_id: Option<&str>, sender: &Sender<ServerEvent>) -> anyhow::Result<()> {
        let redis_key = if let Some(rid) = room_id {
            format!("ws:room:{}:user:{}:unseen", rid, user_id)
        } else {
            format!("ws:user:{}:unseen", user_id)
        };

        let mut unseen: Vec<WsMessage> = self.redis.lrange(&redis_key, 0, -1).await.unwrap_or_default();

        if unseen.is_empty() {
            let sql = "SELECT id, msg_type, room_id, sender_id, recipient_id, content, created_at as timestamp
                       FROM ws_unseen
                       WHERE user_id = $1 AND room_id IS NOT DISTINCT FROM $2
                       ORDER BY created_at ASC";
            let room_param = room_id.map(|s| s.to_string());
            unseen = self.db.query(sql, &[&user_id, &room_param]).await.unwrap_or_default();

            if !unseen.is_empty() {
                let del_sql = "DELETE FROM ws_unseen WHERE user_id = $1 AND room_id IS NOT DISTINCT FROM $2";
                let _ = self.db.execute(del_sql, &[&user_id, &room_param]).await;
            }
        }

        for msg in unseen {
            let serialized = Arc::new(serde_json::to_string(&msg)?);
            let _ = sender.try_send(ServerEvent::Message(serialized));
        }

        self.redis.del(&redis_key).await.ok();
        Ok(())
    }
}

#[get("/ws/{user_id}")]
pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    framework: web::Data<WsService>, // Fixed generic extractor
    path: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let user_id = path.into_inner();
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;
    let framework = framework.get_ref().clone();

    // Changed to Bounded channel
    let (tx, mut rx) = tokio::sync::mpsc::channel::<ServerEvent>(256);

    if let Err(e) = framework.connect(user_id.clone(), tx).await {
        tracing::error!("WS connect failed for {}: {}", user_id, e);
    }

    let user_id_clone = user_id.clone();
    
    actix_web::rt::spawn(async move {
        let mut last_pong = tokio::time::Instant::now();
        let timeout = tokio::time::Duration::from_secs(120);

        loop {
            let deadline = tokio::time::sleep_until(last_pong + timeout);
            tokio::pin!(deadline);

            tokio::select! {
                frame = msg_stream.next() => {
                    match frame {
                        Some(Ok(Message::Text(text))) => {
                            last_pong = tokio::time::Instant::now();
                            match serde_json::from_str::<ClientAction>(&text) {
                                Ok(action) => {
                                    if let Err(e) = framework.handle_action(&user_id_clone, action).await {
                                        tracing::error!("Action error: {}", e);
                                        let _ = session.text(serde_json::json!({"error": e.to_string()}).to_string()).await;
                                    }
                                }
                                Err(e) => {
                                    let _ = session.text(serde_json::json!({"error": format!("Bad JSON: {}", e)}).to_string()).await;
                                }
                            }
                        }
                        Some(Ok(Message::Ping(bytes))) => {
                            let _ = session.pong(&bytes).await;
                        }
                        Some(Ok(Message::Pong(_))) => {
                            last_pong = tokio::time::Instant::now();
                        }
                        Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                        _ => {}
                    }
                }
                event = rx.recv() => {
                    let text = match event {
                        // Dereferences the Arc<String> into a clone without re-serializing
                        Some(ServerEvent::Message(msg)) => msg.as_ref().clone(), 
                        Some(ServerEvent::Error(e)) => serde_json::json!({"error": e}).to_string(),
                        Some(ServerEvent::Pong) => serde_json::json!({"type":"pong"}).to_string(),
                        Some(ServerEvent::Disconnect) | None => break,
                    };
                    if session.text(text).await.is_err() {
                        break;
                    }
                }
                _ = &mut deadline => {
                    tracing::info!("Client timeout for {}", user_id_clone);
                    break;
                }
            }
        }
        
        // CRITICAL: Ensure database flush gets called when loop ends
        let _ = framework.disconnect(&user_id_clone).await;
    });

    Ok(res)
}
