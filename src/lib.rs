// These all are available to prelude already, just simply use.
// Feels easy
pub mod clean;
pub mod crypto;
pub mod db;
pub mod email;
pub mod otp;
pub mod res;
pub mod server;
pub mod utils;
pub mod jwt;
pub mod redis;
pub mod linkv;
pub mod ddos;
pub mod ws;

// Feels easy
pub mod prelude {

    // =========================
    // External crates
    // =========================

    pub use actix_web;
    pub use actix_ws;
    pub use aes_gcm;
    pub use anyhow;
    pub use argon2;
    pub use base64;
    pub use dashmap;
    pub use deadpool_postgres;
    pub use dotenvy;
    pub use dotenvy::{dotenv};
    pub use futures_util;
    pub use futures_util::StreamExt;
    pub use hmac;
    pub use lettre;
    pub use rand;
    pub use serde_json;
    pub use serde_json::json;
    pub use serde;
    pub use sha2;
    pub use tokio;
    pub use tokio_postgres;
    pub use uuid;
    pub use actix_multipart;
    pub use actix_multipart::{
        Multipart
    };

    pub use deadpool_redis;
    
    // =========================
    // Actix Web
    // =========================

    pub use actix_web::{
        HttpRequest, HttpServer,App,  Responder, delete, get, patch, post, put, rt,
        web::{Form, Json, Path, Payload, Query, ServiceConfig, Data as AppData},
    };

    // =========================
    // Serde
    // =========================

    pub use serde::{Deserialize, Serialize};

    // =========================
    // STD
    // =========================

    pub use std::{collections::HashMap, env, io::Result, sync::Arc};

    // =========================
    // Server
    // =========================

    pub use crate::server;
    pub use crate::server::{env_var, env_var_or_default, init_env};

    // =========================
    // Crypto
    // =========================

    pub use crate::crypto;
    pub use crate::crypto::{CryptoService};

    // =========================
    // OTP
    // =========================

    pub use crate::otp;
    pub use crate::otp::{OtpService, OtpConfig};

    // =========================
    // Email
    // =========================

    pub use crate::email;
    pub use crate::email::{EmailService, EmailConfig, EmailData};

    // =========================
    // DB
    // =========================

    pub use crate::db;
    pub use crate::db::{DbPool};

    // =========================
    // Web Socket
    // =========================

    pub use crate::ws;
    pub use crate::ws::{WsManager, WsConfig, UserMsg};

    // =========================
    // JWT Token
    // =========================
    pub use crate::jwt;
    pub use crate::jwt::{Jwt};
    
    // =========================
    // Responses
    // =========================

    pub use crate::res;
    pub use crate::res::{
        http_bad, http_bad_json, http_ok, http_ok_json, send_file, send_json, send_str, upload_file
    };

    // =========================
    // Clean Looks
    // =========================

    pub use crate::clean;
    pub use crate::clean::{Rlt, Rsp, RltRsp, MainRlt, FileRlt, Rqs};
    
        
    // =========================
    // Redis
    // =========================
    pub use crate::redis;
    pub use crate::redis::{
        RedisManager,
    };
    
    // =========================
    // Utils
    // =========================
    pub use crate::utils;
    pub use crate::utils::{
        utils::{input, generate_token},
        index_file::index_file,
        config::app_config
    };

    // =========================
    // Link Verification
    // =========================
    pub use crate::linkv;
    pub use crate::linkv::{
        LinkV,
        LinkVConfig
    };

    // =========================
    // DDoS Protection
    // =========================
    pub use crate::ddos;
    pub use crate::ddos::{
        DdosConfig,
        DdosShield
    };
}

