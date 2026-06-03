# fr-rust Documentation
**fr-rust** is a comprehensive, high-performance web framework utility library built on top of Actix-Web. It provides out-of-the-box support for DDoS protection, standardized responses, WebSockets, Redis, Database pooling, Cryptography, and common verification services (JWT, OTP, Link Verification, Email).
## 1. Quick Start & Configuration
Set up your server, initialize shared states, and protect your app with the built-in DDoS shield in your main entry point.
```rust
use fr_rust::prelude::*;
// Use actix web
use actix_web::{
    App, HttpServer, web::Data as AppData, web
};
#[fr_rust::main]
async fn main() -> MainRlt {
    // 1. Load Environment Variables
    load_env();

    // 2. Configure DDoS Shield
    let ddos_shield = DdosShield::builder()
        .max_requests(5)          // Max requests per window
        .window_secs(1)           // Time window (1 second)
        .ban_duration_secs(20)    // Ban duration for violators
        .block_agent("malicious-bot")
        .allow_missing_ua(false)
        .build();

    // 3. Initialize Shared Services
    let jwt = Jwt::new();
    
    let email_config = EmailConfig {
        smtp_host: env_var("SMTP_HOST"),
        smtp_port: env_var("SMTP_PORT").parse().expect("Invalid SMTP_PORT"),
        smtp_user: env_var("SMTP_USER"),
        smtp_pass: env_var("SMTP_PASS"),
        from_name: env_var("FROM_NAME"),
        from_email: env_var("FROM_EMAIL"),
    };
    let email_service = EmailService::new(email_config).unwrap();
    
    let pool = DbPool::new(env_var("DATABASE_URL"));
    let redis = RedisManager::new(&env_var("REDIS_URL")).unwrap();
    
    let key = env_var("AES_KEY");
    let key_bytes: &[u8; 32] = key.as_bytes().try_into().expect("AES_KEY must be 32 bytes");
    let crypto_service = CryptoService::new(key_bytes).unwrap();
    
    let otp_service = OtpService::new(OtpConfig {
        secret: env_var("KEY"),
        crypto: crypto_service.clone(),
        redis: redis.clone(),
        ttl_secs: 300 
    });
    
    let linkv_service = LinkV::new(LinkVConfig {
        secret: env_var("KEY"),
        crypto: crypto_service.clone(),
        redis: redis.clone(),
        ttl_secs: 300 
    });
    
    let ws = WsManager::new(WsConfig { server: 1, redis: redis.clone() });

    // 4. Start the HTTP Server
    let address = format!("{}:{}", env_var_or_default("IP", "0.0.0.0"), env_var_or_default("PORT", "8080"));
    println!("Starting server at http://{}", address);
    
    HttpServer::new(move || App::new()
        .app_data(AppData::new(email_service.clone()))
        .app_data(AppData::new(pool.clone()))
        .app_data(AppData::new(redis.clone()))
        .app_data(AppData::new(crypto_service.clone()))
        .app_data(AppData::new(otp_service.clone()))
        .app_data(AppData::new(linkv_service.clone()))
        .app_data(AppData::new(jwt.clone()))
        .app_data(AppData::new(ws.clone()))
        .configure(app_config)
        .wrap(ddos_shield.clone())
    )
    .bind(address)?
    .run()
    .await
}

// Route Configuration
pub fn app_config(cfg: &mut web::ServiceConfig) {
    cfg.service(index_file);
}

```
## 2. Common Types
**fr-rust** exports convenient type aliases to reduce boilerplate:
| Type Alias | Original Type | Description |
|---|---|---|
| Rsp | HttpResponse | Standard HTTP Response |
| Rqs | HttpRequest | Standard HTTP Request |
| Rlt | Result<(), actix::Error> | Standard Actix Result |
| MainRlt | (Varies) | Main Function Result |
| FileRlt | (Varies) | File Streaming Result |
## 3. Responses & Routing
**fr-rust** provides utility functions to return standard HTTP responses, stream files, and parse JSON easily.
### File Streaming
Stream large files directly to the client with ease.
```rust
#[get("/")]
pub async fn index_file() -> FileRlt {
    send_file("./static/index.html").await
}

```
### Standard & JSON Responses
| Response Helper | Purpose | Example |
|---|---|---|
| http_ok(msg) | 200 OK with string | http_ok("Success") |
| http_bad(msg) | 400 Bad Request with string | http_bad("Error") |
| send_str(msg) | Raw string response | send_str("Hello") |
| send_json(data) | Standard JSON response (Vec/Struct) | send_json(vec![1, 2]) |
| http_ok_json(data) | 200 OK with JSON map/macro | http_ok_json(json!({"a": 1})) |
| http_bad_json(data) | 400 Bad Request with JSON | http_bad_json(json!({"err": true})) |
**Implementation Example:**
```rust
#[get("/test/responses/{type}")]
async fn test_responses(path: Path<String>) -> Rsp {
    match path.into_inner().as_str() {
        "ok" => http_ok("Ok!"),
        "bad" => http_bad("Error!"),
        "str" => send_str("Hello from send_str!"),
        "json_struct" => send_json(User { id: 1, name: "Sayed".to_string() }),
        "json_vec" => send_json(vec![1, 2, 3]),
        "json_macro_bad" => http_bad_json(json!({"success": false})),
        "json_map" => {
            let mut map = HashMap::new();
            map.insert("name", "Sayed");
            http_ok_json(map)
        },
        _ => http_bad("Unknown response type requested.")
    }
}

```
## 4. WebSockets
Built on top of actix-ws, the WebSocket manager (WsManager) provides robust room management and messaging utilities.
**Core Methods:**
```rust
// Use actix-ws with this manager
// 1. Create a high-performance unbounded channel for a user connection
use tokio::sync::mpsc;
let (tx, mut rx) = mpsc::channel::<String>(128);

// 2. Manage Users
ws_manager.register(user_id, tx);
ws_manager.drop_user(user_id);

// 3. Manage Rooms
ws_manager.join_room(user_id, room_name);
ws_manager.leave_room(user_id, room_name);
ws_manager.drop_room(room_name); // Note: Renamed from drop_user(room_name) to clarify intent

// 4. Send Messages
ws_manager.msg_user(user_id, "Hello User!".to_string());
ws_manager.msg_room(room_name, UserMsg::new("SenderID", "RoomID", "Hello Room!"));
ws_manager.broadcast("System Maintenance in 5 minutes!".to_string());

// 5. Query Rooms
let messages = ws_manager.get_room_msgs(room_name);

// Use the manager in actix route
#[get("/ws/{user_id}")]
async fn ws_handler(
    req: HttpRequest,
    body: web::Payload,
    ws_manager: web::Data<WsManager>,
    path: web::Path<String>,
) -> Rsp {
    let user_id = path.into_inner();

    // 1. Setup high-performance bounded channel (128 items is ideal for memory/backpressure balance)
    let (tx, mut rx) = mpsc::channel::<String>(128);

    // Perform WebSocket handshake
    let (res, mut session, mut msg_stream) = match actix_ws::handle(&req, body) {
        Ok(res) => res,
        Err(_) => return http_bad("Internal Server Error!"), 
    };

    // 2. Register user with the manager
    ws_manager.register(&user_id, tx);
    // here your code.....
}
```
## 5. Database Operations
Access your relational database easily via AppData<DbPool>.
 * **execute:** Run queries without expecting a return dataset (CREATE, INSERT, UPDATE).
 * **query:** Fetch multiple rows.
 * **query_one:** Fetch exactly one row.
 * **query_opt:** Fetch an optional row (returns Option<Row>).
```rust
#[get("/test/db")]
async fn test_db(pool: AppData<DbPool>) -> Rsp {
    // Execution
    pool.execute("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT);", &[]).await.unwrap();
    pool.execute("INSERT INTO users (name) VALUES ($1);", &[&"Alice"]).await.unwrap();
    
    // Multiple Results
    let rows = pool.query("SELECT id, name FROM users;", &[]).await.unwrap();
    
    // Optional Result (Safe Fallback)
    let maybe_row = pool.query_opt("SELECT name FROM users WHERE id = $1;", &[&999]).await.unwrap();
    let fallback = match maybe_row {
        Some(r) => r.get("name"),
        None => "User 999 does not exist.".to_string(),
    };
    
    http_ok("DB Operations successful!")
}

```
## 6. Redis Integrations
Injected via AppData<RedisManager>, the Redis client supports standard operations, TTL, Hashes, Lists, Sets, and coordinated batch Pub/Sub.
**Pub/Sub Example:**
```rust
// Another operations same as deadpool_redis or redis-rs
// It just helps you to create redis pool auto & handle pub/sub.
use deadpool_redis::redis::AsyncCommands;
use futures_util::StreamExt;

let redis = redis_manager.get_connection().await.unwrap();

// Publish a message
redis.publish("event_name", "content").await.unwrap();

// Subscribe to a stream
let mut stream = redis.subscribe("event_name").await?;
while let Some(msg) = stream.next().await {
    let payload: String = msg.get_payload()?;
    println!("Received: {}", payload);
}

```
## 7. Verification & Notification Services
### 7.1 JSON Web Tokens (JWT)
```rust
let secret = "my_ultra_secure_secret_key_2026";
let user_id = "user_12345";

// Generate token (No expiration)
let forever_token = jwt.generate_token(user_id, secret).unwrap();

// Generate token (Expiring)
let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as usize;
let expiry_timestamp = current_time + 3600; // 1 hour from now
let expiring_token = jwt.generate_exp_token(user_id, secret, expiry_timestamp).unwrap();

// Verify
let is_valid = jwt.verify_token(&forever_token, secret);

```
### 7.2 OTP Generation & Verification
Access via AppData<OtpService>.
```rust
let otp = otp_service.generate_otp("user123", 6).await.unwrap(); // 6-digit OTP
if otp_service.verify_otp("user123", &otp).await.unwrap() {
    http_ok("Valid OTP!")
}

```
### 7.3 Link Verification Tokens
Access via AppData<LinkV>.
```rust
let token = linkv_service.generate_token("user123").await.unwrap();
if linkv_service.verify_token("user123", &token).await.unwrap() {
    http_ok("Valid Token!")
}

```
### 7.4 Email Service
Access via AppData<EmailService>.
```rust
#[get("/test/email")]
async fn test_email(email_service: AppData<EmailService>) -> Rsp {
    let data = EmailData {
        to: "receiver@example.com".to_string(),
        subject: "Hello!".to_string(),
        body: "Mail body here.".to_string(),
    };
    
    match email_service.send_email(&data).await {
        Ok(_) => http_ok("Sent!"),
        Err(_) => http_bad("Failed."),
    }
}

```
## 8. Cryptography Service
Inject AppData<CryptoService> to securely encrypt data or hash passwords.
 * **Symmetric Encryption:** encrypt_text / decrypt_text
 * **Password Hashing:** hash_data / verify_hash (Async, highly secure)
 * **Fast Hashing:** sha256_hash
```rust
#[get("/test/crypto")]
async fn test_crypto(crypto: AppData<CryptoService>) -> Rsp {
    // Symmetric Encryption
    let encrypted = crypto.encrypt_text("Hello").unwrap();
    let decrypted = crypto.decrypt_text(&encrypted.encrypted_text).unwrap();
    
    // Fast Hash
    let fast_hashing = crypto.sha256_hash("password").unwrap();
    
    // Secure Password Hashing (Slower)
    let hashed = crypto.hash_data("password").await.unwrap();
    let is_valid = crypto.verify_hash("password", &hashed.hash).await.unwrap();
    
    http_ok("Crypto operations completed.")
}

```
## 9. General Utilities
Standalone utility macros and functions for rapid development.
```rust
// Capture user input directly from the terminal (Python-like)
let name = input("What's your name? ");

// Generate a random HEX-encoded key of a specific byte length
let key = generate_key(100); // 100 character random string
```