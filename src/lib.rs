// Module Declarations
pub mod crypto;
pub mod db;
pub mod ddos;
pub mod email;
pub mod jwt;
pub mod linkv;
pub mod otp;
pub mod redis;
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
    pub use crate::server::{self, env_var, env_var_or_default, load_env};
    pub use crate::types::{self, FileRlt, Main, Rlt, RltRsp, Rsp, Rqs};
    pub use crate::ws::{self, UserMsg, WsConfig, WsManager};
    pub use crate::{
        err, get, post, put, delete, patch, head, options, scope, resource, run_server, setup_jwt
    };
    pub use crate::res::{
          self,
          http_ok_static,
          http_ok_stream,
          http_no_content,
          http_created,
          http_accepted,
          http_partial_content,
          http_bad_static,
          http_unauthorized,
          http_forbidden,
          http_not_found,
          http_method_not_allowed,
          http_unsupported_media,
          http_too_many_requests,
          http_service_unavailable,
          http_server_error,
          send_file_fast,
          stream_file_chunked,
          send_file_range,
          http_brotli,
          http_lz4,
          parse_multipart_stream,
          parse_json_fast,
          parse_range,
          upload_with_progress,
          upload_streaming
    };
    pub use web_main;
}
