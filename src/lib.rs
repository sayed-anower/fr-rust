// Module Declarations
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
pub mod ws;
pub mod macros;
pub mod req;
pub mod res;
use actix_web::main as web_main;

// The Framework Prelude
pub mod prelude {
    // Internal Library Re-exports
    pub use crate::crypto::{self, CryptoService};
    pub use crate::db::{self, DbPool};
    pub use crate::ddos::{self, DdosConfig, DdosShield};
    pub use crate::email::{self, EmailConfig, EmailData, EmailService};
    pub use crate::jwt::{self, Jwt};
    pub use crate::linkv::{self, LinkV, LinkVConfig};
    pub use crate::otp::{self, OtpConfig, OtpService};
    pub use crate::redis::{self, RedisManager, RedisManagerError};
    pub use crate::res::{
        self, http_bad, http_bad_json, http_ok, http_ok_json, send_file, send_json, send_str,
        upload_file,
    };
    pub use crate::server::{self, env_var, env_var_or_default, load_env};
    pub use crate::types::{self, FileRlt, Main, Rlt, RltRsp, Rsp, Rqs};
    pub use crate::ws::{self, UserMsg, WsConfig, WsManager};
    pub use crate::{
        err, get, post, put, delete, patch, head, options, scope, resource, run_server
    };
    pub use crate::res::{
        *
    };
}
