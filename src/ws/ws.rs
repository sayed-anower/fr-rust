use crate::prelude::*;
use deadpool_redis::redis::AsyncCommands;
use anyhow::{Context, Result};
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserMsg {
    pub from: String, // user id
    pub to: String,   // room_name
    pub msg: String,  // message content
    pub time: String, // timestamp
}

impl UserMsg {
    pub fn new(from: String, to: String, msg: String) -> Self {
        Self {
            from,
            to,
            msg,
            time: Utc::now().to_rfc3339(),
        }
    }
}

pub struct WsConfig {
    pub server: u32,
    pub redis: RedisManager,
}

#[derive(Clone)]
pub struct WsManager {
    pub server: u32,
    pub redis: RedisManager,
    // Local state: Maps uid -> Sender channel to the actual WebSocket stream
    pub local_sessions: Arc<DashMap<String, mpsc::Sender<String>>>,
}

impl WsManager {
    // 1. "new" create a new web socket service
    pub fn new(config: WsConfig) -> Self {
        Self {
            server: config.server,
            redis: config.redis,
            local_sessions: Arc::new(DashMap::new()),
        }
    }

    // 2. "register" save new in redis: user_id: server
    pub async fn register(&self, uid: &str, tx: mpsc::Sender<String>) -> anyhow::Result<()> {
        let mut conn = self.redis.get_connection().await?;

        conn.set(format!("user:{}", uid), self.server.to_string()).await?;

        self.local_sessions.insert(uid.to_string(), tx);
        Ok(())
    }

    // 3. "create_room" create new room in redis
    pub async fn create_room(&self, room_name: &str) -> anyhow::Result<()> {
        let mut conn = self.redis.get_connection().await?;
        conn.set(format!("room_exists:{}", room_name), "1").await?;
        Ok(())
    }

    // 4. "join_room" add new user_id to room users.
    pub async fn join_room(&self, room_name: &str, uid: &str) -> Result<()> {
        let mut conn = self.redis.get_connection().await?;
        conn.sadd(format!("room:{}", room_name), uid).await?;
        Ok(())
    }

    // 5. "msg_room" loop in room_users, send msg, save msg in redis
    pub async fn msg_room(&self, room_name: &str, msg_obj: UserMsg) -> Result<()> {
        let mut conn = self.redis.get_connection().await?;
        let msg_str = serde_json::to_string(&msg_obj)?;

        // Save message to room history (Redis List using RPUSH)
        conn.rpush(format!("room_msgs:{}", room_name), &msg_str).await?;

        // Get all users in the room
        let users: Vec<String> = conn.smembers(format!("room:{}", room_name)).await?;

        // Send to each user
        for uid in users {
            // We clone msg_str so we don't consume it
            let _ = self.msg_user(&uid, msg_str.clone()).await; 
        }
        Ok(())
    }

    // 6. "msg_user" take user id, check server match -> send locally OR publish
    pub async fn msg_user(&self, uid: &str, msg: String) -> anyhow::Result<()> {
        let mut conn = self.redis.get_connection().await?;
        
        // Fetch user server data from Redis
        let user_data: Option<String> = conn.get(format!("user:{}", uid)).await?;
        
        if let Some(data) = user_data {
            let server_id = data.parse::<u32>().unwrap_or(0);

            if server_id == self.server {
                // Match! User is connected to THIS server instance. Send directly.
                if let Some(sender) = self.local_sessions.get(uid) {
                    let _ = sender.send(msg).await;
                }
            } else {
                // Doesn't match. User is on another node. Publish to Redis.
                // We wrap it so the receiving server knows who the target is.
                let payload = json!({
                    "target_uid": uid,
                    "msg": msg
                }).to_string();
                
                conn.publish("fr-ws", payload).await?;
            }
        }
        Ok(())
    }

    // 7. "drop_user" remove user from redis and local sessions
    pub async fn drop_user(&self, uid: &str) -> Result<()> {
        let mut conn = self.redis.get_connection().await?;
        conn.del(format!("user:{}", uid)).await?;
        self.local_sessions.remove(uid);
        Ok(())
    }

    // 8. "drop_room" remove room and messages from redis
    pub async fn drop_room(&self, room_name: &str) -> Result<()> {
        let mut conn = self.redis.get_connection().await?;
        conn.del(format!("room:{}", room_name)).await?;
        conn.del(format!("room_msgs:{}", room_name)).await?;
        conn.del(format!("room_exists:{}", room_name)).await?;
        Ok(())
    }

    // 9. "broadcast" loop in all users & send them all msg
    pub async fn broadcast(&self, msg: String) -> Result<()> {
        let mut conn = self.redis.get_connection().await?;
        
        // 1. Publish to the global broadcast channel so ALL servers get it
        conn.publish("fr-ws-broadcast", &msg).await?;
        
        // 2. Send to all users connected to THIS local server immediately
        for entry in self.local_sessions.iter() {
            let _ = entry.value().send(msg.clone()).await;
        }
        Ok(())
    }

    // 10. "get_room_msgs" get all msgs that exist in room_name
    pub async fn get_room_msgs(&self, room_name: &str) -> Result<Vec<UserMsg>> {
        let mut conn = self.redis.get_connection().await?;
        let msgs_str: Vec<String> = conn.lrange(format!("room_msgs:{}", room_name), 0, -1).await?;
        
        let mut msgs = Vec::new();
        for m in msgs_str {
            if let Ok(parsed) = serde_json::from_str(&m) {
                msgs.push(parsed);
            }
        }
        Ok(msgs)
    }
}

