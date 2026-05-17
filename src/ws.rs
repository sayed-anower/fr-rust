use actix_ws::Session;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

// 1. Define your Enum for multiple message types
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AppMessage {
    Notification { title: String, body: String },
    DirectMessage { from: String, content: String },
    SystemAlert(String),
}

// 2. The core WebSocket Manager
#[derive(Clone)]
pub struct WsManager {
    // Outer DashMap: user_id -> Inner DashMap: session_id -> sender channel
    connections: Arc<DashMap<String, DashMap<Uuid, mpsc::Sender<AppMessage>>>>,
}

impl WsManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    /// Registers a new WebSocket session.
    /// Returns a SessionGuard which automatically cleans up the session when dropped.
    pub fn register(&self, user_id: &str, mut session: Session) -> SessionGuard {
        let session_id = Uuid::new_v4();
        let (tx, mut rx) = mpsc::channel::<AppMessage>(100);

        // Add to state
        let user_sessions = self
            .connections
            .entry(user_id.to_string())
            .or_insert_with(DashMap::new);
        user_sessions.insert(session_id, tx);

        // Spawn a dedicated write task for this session
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(text) = serde_json::to_string(&msg) {
                    // If sending fails, the connection is likely dead. Break the loop.
                    if session.text(text).await.is_err() {
                        break;
                    }
                }
            }
        });

        SessionGuard {
            manager: self.clone(),
            user_id: user_id.to_string(),
            session_id,
        }
    }

    // --- Sending Controls ---

    /// Send to a single user (hits ALL of their active sessions)
    pub fn send_to_user(&self, user_id: &str, msg: AppMessage) {
        if let Some(user_sessions) = self.connections.get(user_id) {
            for session in user_sessions.iter() {
                // try_send is non-blocking and highly performant
                let _ = session.value().try_send(msg.clone());
            }
        }
    }

    /// Send to an array of specific user IDs
    pub fn send_to_users(&self, user_ids: &[&str], msg: AppMessage) {
        for id in user_ids {
            self.send_to_user(id, msg.clone());
        }
    }

    /// Broadcast to absolutely everyone connected
    pub fn broadcast(&self, msg: AppMessage) {
        for user_entry in self.connections.iter() {
            for session in user_entry.value().iter() {
                let _ = session.value().try_send(msg.clone());
            }
        }
    }

    // Internal cleanup called by SessionGuard
    fn remove_session(&self, user_id: &str, session_id: Uuid) {
        if let Some(user_sessions) = self.connections.get(user_id) {
            user_sessions.remove(&session_id);
            // If user has no more active sessions, remove the user entirely to free memory
            if user_sessions.is_empty() {
                drop(user_sessions);
                self.connections.remove(user_id);
            }
        }
    }
}

// 3. RAII Guard for automatic cleanup on disconnect
pub struct SessionGuard {
    manager: WsManager,
    user_id: String,
    session_id: Uuid,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        self.manager.remove_session(&self.user_id, self.session_id);
    }
}

pub fn impl_ws(cfg: &mut ServiceConfig) {
    // Initialize the manager
    let ws_manager = WsManager::new();
    let app_data = web::Data::new(ws_manager.clone());
    cfg.app_data(ws_manager);
}
