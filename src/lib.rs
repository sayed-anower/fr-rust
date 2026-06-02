// ==========================================
// 1. Module Declarations
// ==========================================

pub mod crypto;
pub mod db;
pub mod ddos;
pub mod email;
pub mod jwt;
pub mod linkv;
pub mod otp;
pub mod redis;
pub mod res;
pub mod server;
pub mod types;
pub mod utils;
pub mod ws;

// ==========================================
// 2. The Framework Prelude
// ==========================================
pub mod prelude {

    // --------------------------------------
    // External Re-exports
    // --------------------------------------
    
    pub use actix_ws;
    pub use aes_gcm;
    pub use anyhow; 
    pub use argon2;
    pub use base64; 
    pub use dashmap; 
    pub use dotenvy::{self, dotenv}; 
    pub use serde::{self, Deserialize, Serialize}; 
    pub use serde_json::{self, json}; 
    pub use tokio; 
    pub use deadpool_postgres;
    pub use tokio_postgres;
    pub use deadpool_redis::{self, redis::AsyncCommands};
    pub use futures_util::{self, StreamExt};
    pub use actix_multipart::{self, Multipart};
    pub use actix_web::{
        self, delete, get, patch, post, put, rt, web, App, HttpRequest, HttpServer, Responder, main,
        web::{Form, Json, Path, Payload, Query, ServiceConfig, Data as AppData},
    };
    pub use hmac;
    pub use sha2;
    pub use rand;
    pub use lettre;
    pub use uuid; 

    // --------------------------------------
    // Standard Library Re-exports
    // --------------------------------------
    pub use std::{collections::HashMap, env, io::Result as IoResult, sync::Arc};

    // --------------------------------------
    // Internal Library Re-exports
    // --------------------------------------
    pub use crate::crypto::{self, CryptoService};
    pub use crate::db::{self, DbPool};
    pub use crate::ddos::{self, DdosConfig, DdosShield};
    pub use crate::email::{self, EmailConfig, EmailData, EmailService};
    pub use crate::jwt::{self, Jwt};
    pub use crate::linkv::{self, LinkV, LinkVConfig};
    pub use crate::otp::{self, OtpConfig, OtpService};
    pub use crate::redis::{self, RedisManager};
    pub use crate::res::{
        self, http_bad, http_bad_json, http_ok, http_ok_json, send_file, send_json, send_str,
        upload_file,
    };
    pub use crate::server::{self, env_var, env_var_or_default, init_env};
    pub use crate::types::{self, FileRlt, MainRlt, Rlt, RltRsp, Rps, Rqs};
    pub use crate::ws::{self, UserMsg, WsConfig, WsManager};
    pub use crate::utils::{
        self,
        utils::{generate_token, input},
    };
}
