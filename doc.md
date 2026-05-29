### Import it.
```rust
use fr_rust::prelude::*;
```
### Create a server
```rust
#[fr_rust::main]
async fn main() -> MainRlt {
    // Initialized env
    init_env();

    // Load IP and PORT from environment or use defaults
    let ip = env_var("IP", "127.0.0.1");
    let port = env_var("PORT", "8080");
    let address = format!("{}:{}", ip, port);
    
    println!("Starting server at http://{}", address);
    
    // run the server
    let _ = run_server(app_config, &address)?.await;
    
    Ok(())
}
```
### App Config
```rust
use crate::*;
use fr_rust::prelude::*;
// APP CONFIGURATION
// You must pass it to run_server function. run_server(app_config, &address)
pub fn app_config(cfg: &mut ServiceConfig) {
    // Registering the shared state
    
    // Email
    let email_config = EmailConfig {
        smtp_host: env_var("SMTP_HOST"),
        smtp_port: env_var("SMTP_PORT").parse().expect("SMTP_PORT must be a valid integer"),
        smtp_user: env_var("SMTP_USER"),
        smtp_pass: env_var("SMTP_PASS"),
        from_name: env_var("FROM_NAME"),
        from_email: env_var("FROM_EMAIL"),
    };
    let email_service =
    EmailService::new(email_config).unwrap();
    
    // Database
    let database_url = env_var("DATABASE_URL");
    
    // Create pool
    let pool = DbPool::new(database_url);
    
    // Redis
    // Redis url
    let redis_url = env_var("REDIS_URL");
    // Redis service
    let redis = RedisManager::new(redis_url).await.expect("Failed to connect to Redis");

    // Crypto
    // Get aes key
    let key = env_var("AES_KEY");
    // Convert to bytes
    let key_bytes: &[u8; 32] = key.as_bytes().try_into().expect("AES_KEY must be exactly 32 bytes");
    // Service
    let crypto_service = CryptoService::new(&key).expect("Failed to init crypto");
    
    // Otp
    let otp_key = env_var("KEY");
    let otp_service = OtpService::new(otp_key, redis_url, 300); // 300 sec / 5 min
    
    // Set app data
    cfg
    // For email
    .app_data(AppData::new(email_service))
    // For DB
    .app_data(AppData::new(pool))
    // For redis
    .app_data(AppData::new(redis))
    // For crypto
    .app_data(AppData::new(crypto_service))
    // For otp
    .app_data(AppData::new(otp_service));
    
    // Configured
    cfg.service(index_file)
       .service(test_email)
       .service(test_otp)
       .service(test_crypto)
       .service(test_db)
       .service(ws_route)
       .service(message_group)
       .service(test_responses);
}
```
### A struct that will be used in following examples.
```rust
#[derive(Serialize)]
struct User {
    id: u32,
    name: String,
}
```
### Send responses
```rust
// --- JSON & Standard Response Features ---
#[get("/test/responses/{type}")]
async fn test_responses(path: Path<String>) -> Rsp {
    let res_type = path.into_inner();
    
    match res_type.as_str() {
    // http with string body & status 200
        "ok" => http_ok("Ok!"),
    // http with string body & status 409
        "bad" => http_bad("Error!"),
    // send string body
        "str" => send_str("Hello from send_str!"),
    // send json body
    // Way 1 = send by struct
        "json_struct" => {
            let data = User { id: 1, name: "Sayed".to_string() };
            send_json(data)
        },
    // Way 2 = send by vector
        "json_vec" => send_json(vec![1, 2, 3]),
    // Way 3 = send by json macro from serde_json
        "json_macro_bad" => {
            let js = json!({
                "success": false,
                "message": "Login failed"
            });
            http_bad_json(js)
        },
    // Way 4 = send hashmap
        "json_map" => {
            let mut map = HashMap::new();
            map.insert("name", "Sayed");
            map.insert("role", "Admin");
            http_ok_json(map)
        },
        // http bad with string body & status 400
        _ => http_bad("Unknown response type requested.")
    }
}
```
### Send file
```rust
// Static files
#[get("/")]
async fn index_file() -> FileRlt {
    // Send the file
    send_file("./static/index.html").await
}
```
### Send email
```rust
// Send Email
#[get("/test/email")]
async fn test_email(email_service: AppData<EmailService>) -> Rsp {
    let data = EmailData {
        to: "receiver@example.com",
        subject: "Hello from fr_rust!",
        body: "This is the mail body sent via our custom library.".to_string(),
    };
    
    match email_service.send_email(config, data) {
        Ok(_) => http_ok("Email sent successfully!"),
        Err(_) => http_bad("Failed to send email."),
    }
}
```
### Send otp
```rust
// OTP Feature
#[get("/test/otp")]
async fn test_otp(otp_service: AppData<OtpService>) -> Rsp {
    let user_id = "user123";

    // Generate OTP
    let otp = otp_service.generate_otp(user_id, digit);

    // Verify OTP
    if otp_service.verify_otp(user_id, &otp) {
        http_ok("Valid OTP! Verification passed.")
    } else {
        http_bad("Invalid OTP!")
    }
}
```
### Encrypt, Decrypt & Hash
```rust
// Crypto Feature (Encrypt/Decrypt & Hash/Verify Hash)
#[get("/test/crypto")]
async fn test_crypto(crypto: App Data<CryptoService>) -> Rsp {
    // ENCRYPT & DECRYPT
    let original_text = "Hello world!";
    let encrypted = crypto.encrypt_text(original_text).await.unwrap();
    let decrypted = crypto.decrypt_text(&encrypted.encrypted_text).await.unwrap();
    
    // HASH & VERIFY
    let password = "super_secret_password";
    let hashed = crypto.hash_data(password).await.unwrap();
    let verify_ok = crypto.verify_hash(password, &hashed.hash).await.unwrap();
    
    let result = json!({
        "encrypted": encrypted.encrypted_text,
        "decrypted": decrypted.text,
        "hash": hashed.hash,
        "is_verified": verify_ok
    });
    
    http_ok_json(result)
}
```
### Database operations
```rust
// --- Database Feature ---
#[get("/test/db")]
async fn test_db(pool: AppData<DbPool>) -> Rsp {
    // Execute creation
    pool.execute("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT, age INT);", &[]).await;
    
    // Insert
    let insert_query = "INSERT INTO users (name, age) VALUES ($1, $2);";
    pool.execute(insert_query, &[&"Alice", &30]).await;
    
    // Query Multiple
    let select_all_query = "SELECT id, name, age FROM users;";
    let rows = pool.query(&pool, select_all_query, &[]).await.unwrap();
    let mut users = Vec::new();
    for row in rows {
        let id: i32 = row.get("id");
        let name: String = row.get("name");
        let age: i32 = row.get("age");
        users.push(format!("ID: {}, Name: {}, Age: {}", id, name, age));
    }
    
    // Query One
    let select_one_query = "SELECT name FROM users WHERE id = $1;";
    let row = pool.query_one(select_one_query, &[&1]).await.unwrap();
    let name: String = row.get("name");
    
    // Query Optional
    let select_opt_query = "SELECT name FROM users WHERE id = $1;";
    let maybe_row = pool.query_opt(&pool, select_opt_query, &[&999]).await.unwrap();
    let fallback = match maybe_row {
        Some(r) => r.get("name"),
        None => "User 999 does not exist (Safe fallback!).".to_string(),
    };
    
    let result = json!({
        "all_users": users,
        "user_1": name,
        "user_999_fallback": fallback
    });
    
    http_ok_json(result)
}
```