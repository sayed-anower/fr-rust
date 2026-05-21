use fr_rust::prelude::*;

// User structure
#[derive(Serialize)]
struct User {
    id: u32,
    name: String,
}

// ROUTE HANDLERS

// Static Files
#[get("/")]
async fn index_file() -> FileRlt {
    send_file("./static/index.html").await
}

// Email Feature
#[get("/test/email")]
async fn test_email() -> Rsp {
    let config = EmailConfig {
        smtp_host: env_var("SMTP_HOST","smtp.ex.io"),
        smtp_port: env_var("SMTP_PORT", "000").parse().expect("SMTP_PORT must be a valid integer"),
        smtp_user: env_var("SMTP_USER", "u@ex.io"),
        smtp_pass: env_var("SMTP_PASS", "pwd"),
        from_name: env_var("FROM_NAME", "Unknown"),
        from_email: env_var("FROM_EMAIL", "ex@ex.com"),
    };
    let data = EmailData {
        to: "bluekite1234@gmail.com",
        subject: "Hello from fr_rust!",
        body: "This is the mail body sent via our custom library.".to_string(),
    };
    
    match send_email(config, data) {
        Ok(_) => http_ok("Email sent successfully!"),
        Err(_) => http_bad("Failed to send email."),
    }
}

// OTP Feature
#[get("/test/otp")]
async fn test_otp() -> Rsp {
    let secs = 60;
    let digit = 6;
    let otp_service = OtpService::new(env_var("KEY", "key"), secs);
    let user_id = "user123";
    
    let otp = otp_service.generate_otp(user_id, digit);
    
    if otp_service.verify_otp(user_id, &otp) {
        http_ok("Valid OTP! Verification passed.")
    } else {
        http_bad("Invalid OTP!")
    }
}

// Crypto Feature (Encrypt/Decrypt & Hash/Verify)
#[get("/test/crypto")]
async fn test_crypto() -> Rsp {
    let key = env_var("AES_KEY", "12345678901234567890123456789012");
    let key_bytes: &[u8; 32] = key.as_bytes().try_into().expect("AES_KEY must be exactly 32 bytes");
    
    let config = CryptoConfig {
        encryption_key: key_bytes,
    };
    
    let original_text = "Hello world!";
    let encrypted = encrypt_text(&config, original_text).await.unwrap();
    let decrypted = decrypt_text(&config, &encrypted.encrypted_text).await.unwrap();
    
    let password = "super_secret_password";
    let hashed = hash_data(password).await.unwrap();
    let verify_ok = verify_hash(password, &hashed.hash).await.unwrap();
    
    let result = json!({
        "encrypted": encrypted.encrypted_text,
        "decrypted": decrypted.text,
        "hash": hashed.hash,
        "is_verified": verify_ok
    });
    
    http_ok_json(result)
}

// Database Feature
#[get("/test/db")]
async fn test_db() -> Rsp {
    let database_url = env_var("DATABASE_URL", "postgresql://un:pwd@localhost:5432/db");
    let pool = create_db_pool(database_url);
    
    let _ = db_execute(&pool, "CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT, age INT);", &[]).await;
    
    let insert_query = "INSERT INTO users (name, age) VALUES ($1, $2);";
    let _ = db_execute(&pool, insert_query, &[&"Alice", &30]).await;
    
    let select_all_query = "SELECT id, name, age FROM users;";
    let rows = db_query(&pool, select_all_query, &[]).await.unwrap();
    let mut users = Vec::new();
    for row in rows {
        let id: i32 = row.get("id");
        let name: String = row.get("name");
        let age: i32 = row.get("age");
        users.push(format!("ID: {}, Name: {}, Age: {}", id, name, age));
    }
    
    let select_one_query = "SELECT name FROM users WHERE id = $1;";
    let row = db_query_one(&pool, select_one_query, &[&1]).await.unwrap();
    let name: String = row.get("name");
    
    let select_opt_query = "SELECT name FROM users WHERE id = $1;";
    let maybe_row = db_query_opt(&pool, select_opt_query, &[&999]).await.unwrap();
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

// WebSocket Feature
#[get("/ws/{user_id}")]
async fn ws_route(
    req: Rqs,
    body: Payload,
    manager: AppData<WsManager>,
    path: Path<String>
) -> RltRsp {
    let user_id = path.into_inner();
    let (response, session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let guard = manager.register(&user_id, session);
    println!("User connected: {}", &user_id);
    
    rt::spawn(async move {
        let _keep_alive = guard;
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                actix_ws::Message::Text(text) => {
                    println!("{user_id}: {text}");
                }
                actix_ws::Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    });
    
    Ok(response)
}

#[get("/ws-msg")]
async fn message_group(manager: AppData<WsManager>) -> Rsp {
    let target_users = vec!["user123", "user456", "admin99"];
    
    manager.send_to_users(
        &target_users,
        AppMessage::Notification {
            title: "Group Update".to_string(), 
            body: "Meeting starts in 5 minutes".to_string() 
        }
    );
    
    manager.send_to_user(
        "user123",
        AppMessage::DirectMessage {
            from: "user123".to_string(),
            content: "Hi!".to_string(),
        }
    );
    
    manager.broadcast(AppMessage::SystemAlert("System alert!".to_string()));
    
    http_ok("WebSocket messages sent!")
}

// JSON & Standard Response Features
#[get("/test/responses/{res_type}")]
async fn test_responses(path: Path<String>) -> Rsp {
    let res_type = path.into_inner();
    
    match res_type.as_str() {
        "ok" => http_ok("Ok!"),
        "bad" => http_bad("Error!"),
        "str" => send_str("Hello from send_str!"),
        "json_struct" => {
            let data = User { id: 1, name: "Sayed".to_string() };
            send_json(data)
        },
        "json_vec" => send_json(vec![1, 2, 3]),
        "json_macro_bad" => {
            let js = json!({
                "success": false,
                "message": "Login failed"
            });
            http_bad_json(js)
        },
        "json_map" => {
            let mut map = HashMap::new();
            map.insert("name", "Sayed");
            map.insert("role", "Admin");
            http_ok_json(map)
        },
        _ => http_bad("Unknown response type requested.")
    }
}

// APP CONFIGURATION & MAIN
pub fn app_config(cfg: &mut ServiceConfig) {
    // 1. Instantiating and registering the shared state for your route extraction
    let ws_manager = WsManager::new();
    cfg.app_data(AppData::new(ws_manager));

    // 2. Fixed broken syntax chaining (attached to `cfg`)
    cfg.service(index_file)
       .service(test_email)
       .service(test_otp)
       .service(test_crypto)
       .service(test_db)
       .service(ws_route)
       .service(message_group)
       .service(test_responses);
}

#[fr_rust::main]
async fn main() -> MainRlt {
    init_env();
    
    let ip = env_var("IP", "127.0.0.1");
    let port = env_var("PORT", "8080");
    let address = format!("{}:{}", ip, port);
    
    println!("Starting server at http://{}", address);
    
    let _ = run_server(app_config, &address)?.await;
    
    Ok(())
}

