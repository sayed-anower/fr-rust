// These all are available to prelude already, just simply use.
// Feels easy
pub mod clean;
pub mod config;
pub mod crypto;
pub mod db;
pub mod email;
pub mod otp;
pub mod res;
pub mod routes;
pub mod server;
pub mod ws;
pub use actix_web::main;

// Alias
pub use clean as cl;

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
    pub use futures_util;
    pub use hmac;
    pub use lettre;
    pub use rand;
    pub use serde;
    pub use sha2;
    pub use tokio;
    pub use tokio_postgres;
    pub use uuid;

    // =========================
    // Actix Web
    // =========================

    pub use actix_web::{
        HttpRequest, Responder, delete, get, patch, post, put, rt,
        web::{Data as AppData, Form, Json, Path, Payload, Query, ServiceConfig},
    };

    // =========================
    // Serde
    // =========================

    pub use serde::{Deserialize, Serialize};

    // =========================
    // STD
    // =========================

    pub use std::{collections::HashMap, env, io::Result};

    // =========================
    // Server
    // =========================

    pub use crate::server;
    pub use crate::server::{env_var, init_env, run_server};

    // =========================
    // Crypto
    // =========================

    pub use crate::crypto;

    pub use crate::crypto::{CryptoConfig, decrypt_text, encrypt_text, hash_data, verify_hash};

    // =========================
    // OTP
    // =========================

    pub use crate::otp;

    pub use crate::otp::OtpService;

    // =========================
    // Email
    // =========================

    pub use crate::email;

    pub use crate::email::{EmailConfig, EmailData, send_email};

    // =========================
    // DB
    // =========================

    pub use crate::db;

    pub use crate::db::{DbPool, create_db_pool, db_execute, db_query, db_query_one, db_query_opt};

    // =========================
    // Routes
    // =========================

    pub use crate::routes;

    // =========================
    // Config
    // =========================

    pub use crate::config;

    // =========================
    // WebSocket
    // =========================

    pub use crate::ws;

    pub use crate::ws::{AppMessage, WsManager, impl_ws};

    // =========================
    // Responses
    // =========================

    pub use crate::res;

    pub use crate::res::{
        http_bad, http_bad_json, http_ok, http_ok_json, send_file, send_json, send_str,
    };

    // =========================
    // Clean Looks
    // =========================

    pub use crate::clean as cl;

    pub use crate::cl::{Rlt, RltRsp, Rsp, MainRlt, FileRlt};
}
