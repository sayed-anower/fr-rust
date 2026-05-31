```markdown
# fr-rust Framework Documentation

Welcome to the **fr-rust** framework documentation! This guide covers the complete setup, core features, routing, services, and utilities provided by the framework to help you build fast, secure, and scalable web applications in Rust.

---

## 1. Getting Started

To access all framework features, include the prelude in your files:

```rust
use fr_rust::prelude::*;

```
### 1.1 Environment Variables
To utilize all services fully, ensure the following variables are defined in your .env file:
| Variable | Description |
|---|---|
| IP, PORT | Server binding address (defaults to 0.0.0.0:8080) |
| DATABASE_URL | Connection string for your database pool |
| REDIS_URL | Connection string for Redis cache/pubsub |
| AES_KEY | 32-byte encryption key for CryptoService |
| KEY | Secret key used for OTP and Link Verification |
| SMTP_HOST, SMTP_PORT | SMTP server routing details |
| SMTP_USER, SMTP_PASS | SMTP authentication credentials |
| FROM_NAME, FROM_EMAIL | Sender details for the EmailService |
## 2. Server Configuration (main.rs)
The entry point of your application sets up all shared states, middleware, and dependency injections.
```rust
use fr_rust::prelude::*;

// App Configuration Router
pub fn app_config(cfg: &mut ServiceConfig) {
    // Register routes here
    cfg.service(index_file);
    // cfg.service(test_responses);
    // ...
}

#[actix_web::main]
async fn main() -> MainRlt {
    dotenv().ok();
    
    // --- Middlewares & Security ---
    let ddos_shield = DdosShield::builder()
        .max_requests(5) // Max requests per window
        .window_secs(1)  // Window timeframe (1 second)
        .ban_duration_secs(20)
        .block_agent("malicious-bot")
        .allow_missing_ua(false).build();

    // --- Service Initialization ---
    // Email
    let email_config = EmailConfig {
        smtp_host: env_var("SMTP_HOST"),
        smtp_port: env_var("SMTP_PORT").parse().expect("SMTP_PORT must be a valid integer"),
        smtp_user: env_var("SMTP_USER"),
        smtp_pass: env_var("SMTP_PASS"),
        from_name: env_var("FROM_NAME"),
        from_email: env_var("FROM_EMAIL"),
    };
    let email_service = EmailService::new(email_config).unwrap();
    
    // Database
    let pool = DbPool::new(env_var("DATABASE_URL"));

    // Redis
    let redis_url = env_var("REDIS_URL");
    let redis = RedisManager::new(redis_url).await.unwrap();
    
    // Crypto
    let key = env_var("AES_KEY");
    let key_bytes: &[u8; 32] = key.as_bytes().try_into().expect("AES_KEY must be exactly 32 bytes");
    let crypto_service = CryptoService::new(key_bytes).unwrap();
    
    // OTP & Link Verification
    let otp_config = OtpConfig {
        secret: env_var("KEY"),
        crypto: crypto_service.clone(),
        redis: redis.clone(),
        ttl_secs: 300 
    };
    let otp_service = OtpService::new(otp_config);
    
    let linkv_config = LinkVConfig {
        secret: env_var("KEY"),
        crypto: crypto_service.clone(),
        redis: redis.clone(),
        ttl_secs: 300 
    };
    let linkv_service = LinkV::new(linkv_config);
    
    // WebSockets
    let ws = WsService::new(redis.clone(), pool.clone()); 

    // --- Server Boot ---
    let ip = env_var_or_default("IP", "0.0.0.0");
    let port = env_var_or_default("PORT", "8080");
    let address = format!("{}:{}", ip, port);
    
    println!("Starting server at http://{}", address);
    
    HttpServer::new(move || App::new()
        .app_data(AppData::new(email_service.clone()))
        .app_data(AppData::new(pool.clone()))
        .app_data(AppData::new(redis.clone()))
        .app_data(AppData::new(crypto_service.clone()))
        .app_data(AppData::new(otp_service.clone()))
        .app_data(AppData::new(linkv_service.clone()))
        .app_data(AppData::new(ws.clone()))
        .configure(app_config)
        .wrap(ddos_shield.clone())
    )
    .bind(address)?
    .run()
    .await
}

```
## 3. Responses & Routing
**fr-rust** provides comprehensive macros and utility functions to return standard HTTP responses, stream files, and parse JSON.
### 3.1 File Streaming
Stream large files easily directly to the client.
```rust
#[get("/")]
pub async fn index_file() -> FileRlt {
    send_file("./static/index.html").await
}

```
### 3.2 Standard & JSON Responses
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
## 4. Database Operations
Access your relational database using AppData<DbPool>.
 * **execute**: Run queries without expecting a return dataset (CREATE, INSERT, UPDATE).
 * **query**: Fetch multiple rows.
 * **query_one**: Fetch exactly one row.
 * **query_opt**: Fetch an optional row (returns Option<Row>).
```rust
#[get("/test/db")]
async fn test_db(pool: AppData<DbPool>) -> Rsp {
    // Execution
    pool.execute("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT);", &[]).await;
    pool.execute("INSERT INTO users (name) VALUES ($1);", &[&"Alice"]).await;
    
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
## 5. Redis Integrations
**fr-rust** features a fully-featured async Redis manager injected via web::Data<RedisManager>.
### Key Features
 * **Basic KV & TTL:** set, get, set_ex, exists, ttl
 * **Hashes:** hset, hget, hdel
 * **Lists:** lpush, rpush, lrange
 * **Sets:** sadd, smembers
 * **Pipelines:** Execute multiple commands efficiently via pipeline_set
 * **Cache-Aside Pattern:** cache_or_fetch automatically fetches from DB on cache miss.
 * **Pub/Sub:** Coordinated batched pub/sub for strings and JSON payloads.
**Cache-Aside Example:**
```rust
let fetch_action = || async { "Data From Database".to_string() };
// Misses cache first time, hits cache second time
let res = manager.cache_or_fetch("cache:item", 10, fetch_action).await?;

```
## 6. Verification & Notification Services
### 6.1 Email Service
Send emails easily utilizing the injected EmailService.
```rust
#[get("/test/email")]
async fn test_email(email_service: web::Data<EmailService>) -> Rsp {
    let data = EmailData {
        to: "receiver@example.com".to_string(),
        subject: "Hello!".to_string(),
        body: "Mail body here.".to_string(),
    };
    
    match email_service.send_email(config, data) {
        Ok(_) => http_ok("Sent!"),
        Err(_) => http_bad("Failed."),
    }
}

```
### 6.2 OTP Generation & Verification
```rust
let otp = otp_service.generate_otp("user123", 6);
if otp_service.verify_otp("user123", &otp) {
    http_ok("Valid OTP!")
}

```
### 6.3 Link Verification Tokens
```rust
let token = linkv_service.generate_token("user123");
if linkv_service.verify_token("user123", &token) {
    http_ok("Valid Token!")
}

```
## 7. Cryptography Service
Inject AppData<CryptoService> to safely encrypt data or hash passwords.
 * **Symmetric Encryption:** encrypt_text / decrypt_text
 * **Password Hashing:** hash_data / verify_hash (Async/Secure)
 * **Fast Hashing:** sha256_hash
```rust
#[get("/test/crypto")]
async fn test_crypto(crypto: AppData<CryptoService>) -> Rsp {
    let encrypted = crypto.encrypt_text("Hello").unwrap();
    let decrypted = crypto.decrypt_text(&encrypted.encrypted_text).unwrap();
    
    let hashed = crypto.hash_data("password").await.unwrap();
let fast_hashing = crypto.sha256_hash("password").await.unwrap();
    let is_valid = crypto.verify_hash("password", &hashed.hash).await.unwrap();
    
    http_ok("Crypto operations completed.")
}

```
## 8. General Utilities
Standalone utilities for CLI interactions and fast key generation.
```rust
/// Captures user input directly from the terminal like python
let name = input("What's your name!");

/// Generates a random HEX encoded key of the given byte length
let key = generate_key(100); // 100 length random key

```
