// ==========================================
// 1. Conditional Module Declarations
// ==========================================

#[cfg(feature = "crypto")]
pub mod crypto;

#[cfg(feature = "db")]
pub mod db;

#[cfg(feature = "ddos")]
pub mod ddos;

#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "jwt")]
pub mod jwt;

#[cfg(feature = "linkv")]
pub mod linkv;

#[cfg(feature = "otp")]
pub mod otp;

#[cfg(feature = "redis")]
pub mod redis;

#[cfg(feature = "res")]
pub mod res;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "types")]
pub mod types;

#[cfg(feature = "utils")]
pub mod utils;

#[cfg(feature = "ws")]
pub mod ws;

// ==========================================
// 2. The Conditional Framework Prelude
// ==========================================
pub mod prelude {

    // --------------------------------------
    // External Re-exports (Gated by features)
    // --------------------------------------
    
    #[cfg(feature = "ws")]
    pub use actix_ws;

    #[cfg(feature = "crypto")]
    pub use aes_gcm;

    pub use anyhow; // Core utility, always on

    #[cfg(feature = "crypto")]
    pub use argon2;

    pub use base64; // Core utility, always on
    pub use dashmap; // Core utility, always on
    pub use dotenvy::{self, dotenv}; // Core utility, always on
    pub use serde::{self, Deserialize, Serialize}; // Core utility, always on
    pub use serde_json::{self, json}; // Core utility, always on
    pub use tokio; // Core utility, always on

    #[cfg(feature = "db")]
    pub use deadpool_postgres;
    #[cfg(feature = "db")]
    pub use tokio_postgres;

    #[cfg(feature = "redis")]
    pub use deadpool_redis::{self, redis::AsyncCommands};

    #[cfg(feature = "web")]
    pub use futures_util::{self, StreamExt};
    #[cfg(feature = "web")]
    pub use actix_multipart::{self, Multipart};
    #[cfg(feature = "web")]
    pub use actix_web::{
        self, delete, get, patch, post, put, rt, web, App, HttpRequest, HttpServer, Responder, main,
        web::{Form, Json, Path, Payload, Query, ServiceConfig, Data as AppData},
    };

    #[cfg(feature = "crypto")]
    pub use hmac;
    #[cfg(feature = "crypto")]
    pub use sha2;
    #[cfg(feature = "crypto")]
    pub use rand;

    #[cfg(feature = "email")]
    pub use lettre;

    pub use uuid; // Core utility, always on

    // --------------------------------------
    // Standard Library Re-exports
    // --------------------------------------
    pub use std::{collections::HashMap, env, io::Result as IoResult, sync::Arc};

    // --------------------------------------
    // Internal Library Re-exports (Gated)
    // --------------------------------------
    #[cfg(feature = "crypto")]
    pub use crate::crypto::{self, CryptoService};

    #[cfg(feature = "db")]
    pub use crate::db::{self, DbPool};

    #[cfg(feature = "ddos")]
    pub use crate::ddos::{self, DdosConfig, DdosShield};

    #[cfg(feature = "email")]
    pub use crate::email::{self, EmailConfig, EmailData, EmailService};

    #[cfg(feature = "jwt")]
    pub use crate::jwt::{self, Jwt};

    #[cfg(feature = "linkv")]
    pub use crate::linkv::{self, LinkV, LinkVConfig};

    #[cfg(feature = "otp")]
    pub use crate::otp::{self, OtpConfig, OtpService};

    #[cfg(feature = "redis")]
    pub use crate::redis::{self, RedisManager};

    #[cfg(feature = "res")]
    pub use crate::res::{
        self, http_bad, http_bad_json, http_ok, http_ok_json, send_file, send_json, send_str,
        upload_file,
    };

    #[cfg(feature = "server")]
    pub use crate::server::{self, env_var, env_var_or_default, init_env};

    #[cfg(feature = "types")]
    pub use crate::types::{self, FileRlt, MainRlt, Rlt, RltRsp, Rps, Rqs};

    #[cfg(feature = "ws")]
    pub use crate::ws::{self, UserMsg, WsConfig, WsManager};

    #[cfg(feature = "utils")]
    pub use crate::utils::{
        self,
        utils::{generate_token, input},
    };
}
