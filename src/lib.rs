// These all are available to prelude already, just simply use.
// Feels easy
pub mod server;
pub mod crypto;
pub mod otp;
pub mod email;
pub mod db;
pub mod routes;
pub mod config;
pub mod ws;
pub mod res;
pub mod clean;

// Alias
pub use clean as cl;

// Feels easy
pub mod prelude {

    // =========================
    // External crates
    // =========================

    pub use actix_web;
    pub use actix_ws;
    pub use futures_util;
    pub use dashmap;
    pub use rand;
    pub use tokio;
    pub use dotenvy;
    pub use serde;
    pub use uuid;
    pub use argon2;
    pub use anyhow;
    pub use aes_gcm;
    pub use base64;
    pub use tokio_postgres;
    pub use deadpool_postgres;
    pub use lettre;
    pub use hmac;
    pub use sha2;

    // =========================
    // Actix Web
    // =========================

    pub use actix_web::{
        delete,
        get,
        patch,
        post,
        put,
        main,
        rt,
        HttpRequest,
        Responder,
        web::{
            Json,
            Form,
            Query,
            Path,
            Data as AppData,
            ServiceConfig,
            Payload,
        },
    };

    // =========================
    // Serde
    // =========================

    pub use serde::{
        Serialize,
        Deserialize,
    };

    // =========================
    // STD
    // =========================

    pub use std::{
        env,
        collections::HashMap,
        io::Result,
    };

    // =========================
    // Server
    // =========================

    pub use crate::server;
    pub use crate::server::{
        run_server,
        init_env,
        env_var,
    };

    // =========================
    // Crypto
    // =========================

    pub use crate::crypto;

    pub use crate::crypto::{
        CryptoConfig,
        hash_text,
        encrypt_text,
        decrypt_text,
        verify_hash,
    };

    // =========================
    // OTP
    // =========================

    pub use crate::otp;

    pub use crate::otp::{
        OtpService,
    };

    // =========================
    // Email
    // =========================

    pub use crate::email;

    pub use crate::email::{
        EmailData,
        EmailConfig,
        send_email,
    };

    // =========================
    // DB
    // =========================

    pub use crate::db;

    pub use crate::db::{
        query,
        query_one,
        query_opt,
        execute,
        create_pool,
        DbPool,
    };

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

    pub use crate::ws::{
        impl_ws,
        WsManager,
        AppMessage,
    };

    // =========================
    // Responses
    // =========================

    pub use crate::res;

    pub use crate::res::{
        send_str,
        send_json,
        send_file,
        http_ok,
        http_bad,
        http_ok_json,
        http_bad_json,
    };

    // =========================
    // Clean Looks
    // =========================

    pub use crate::clean as cl;

    pub use crate::cl::{
        Rsp,
        Rlt,
        RltRsp,
    };
}