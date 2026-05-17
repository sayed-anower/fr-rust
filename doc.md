***
* This is optimized to run on local environment & most vps.
***

# Import the lib
```rust
use fr_rust::prelude::*;
```

# Handle server:
```rust
use fr_rust::prelude::*;

// Your app config
pub fn app_config(cfg: &mut ServiceConfig) {
  /* Configurations */
  cfg
  // Add services exact like actix
  .service(index_file);
}

// Send index.html file
// Use macros for clean look.
#[get("/")]
async fn index_file() -> Rsp {
    // Simply send the html file
    send_file("./static/index.html").await
}

#[fr_rust::main]
async fn main() -> Rlt {
  // Initialized env
  init_env(); // Won't crash on vps.
  
  // Port & Address
  // env_var function to access .env file variables
  // It takes 2 params env_var(variable_name, default)
  
  let ip = env_var("SERVER_IP", "127.0.0.1"); // If variable not found, return default = "127.0.0.1"
  
  let port = env_var("SERVER_PORT", "8080");
  
  let address = format!("{}:{}", ip, port);

  // run the server
  run_server(app_config, &address)?.await;
}
```
### Don't get confused
```
* File Structure
/project/
  |-/src/main.rs -> when you access from main.rs : "./static/index.html"
  |-/static/index.html
```


# Send email:
```rust
fn main() {
  let config = EmailConfig {
    smtp_host: "smtp.example.com",
    smtp_port: 587,
    smtp_user: "user@example.com",
    smtp_pass: "password",
    from_name: "John Doe",
    from_email: "john@example.com",
  };

  let data = EmailData {
    to: "receiver@example.com",
    subject: "Hello!",
    body: "This is the mail body".to_string(),
  };

  send_email(config, data)?;
}
```


# Otp Handle:
```rust
fn main() {
  let secs = 60; // 1 minute
  let digit = 6; // 6 digit otp
  let opt_service = OtpService::new("your_secret_key", secs);
  let user_id = "user123";
  let otp = otp_service.generate_otp(user_id, digit);
  if otp_service.verify_otp(user_id, otp) {
    println!("Valid otp!");
  } else {
    println!("Invalid otp!");
  }
}
```

# Handle hashing:
```rust
async fn some_fn() -> anyhow::Result<()> {
    let data = "A password!";

    let hashed = hash_text(data).await?;
    let actual_hash = hashed.hash;
    
    let is_valid = verify_hash(data, &actual_hash).await?;
    
    println!("Is valid: {}", is_valid);

    Ok(())
}
```

# Handle Encrypt/Decrypt:
```rust
async fn some_fn() -> anyhow::Result<()> {
    // Added 'b' for bytes, and made it exactly 32 bytes long
    let secret_key = b"this-key-must-be-exact-32-bytes!"; 
    
    let config = CryptoConfig { 
        encryption_key: secret_key 
    };

    let original_text = "Hello, this is a secret message!";
    println!("Original: {}", original_text);

    // Encrypts and extracts data
    let encrypted = encrypt_text(&config, original_text).await?;
    
    let encrypted_data = encrypted.encrypted_text;
    
    println!("Encrypted (Base64): {}", encrypted_data);

    // Decrypts and extracts data
    let decrypted_data = decrypt_text(&config, &encrypted_data).await?;
    
    let recovered_text = decrypted_data.text;
    
    println!("Decrypted: {}", recovered_text);

    Ok(())
}
```

# Handle DB:
```rust
async fn some_fn() -> anyhow::Result<()> {
    let database_url = "postgresql://username:password@localhost:5432/my_database";
    let pool = create_db_pool(database_url);


    db_execute(&pool, "CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT, age INT);", &[]).await?;

    // db_query_one()
    println!("--- Testing db_query_one() ---");
    let insert_query = "INSERT INTO users (name, age) VALUES ($1, $2);";
    // You can Replace .expect() with ?
    let rows_affected = db_execute(&pool, insert_query, &[&"Alice", &30]).await?;
    println!("Rows inserted: {}\n", rows_affected);


    // db_query()
    println!("--- Testing db_query() ---");
    let select_all_query = "SELECT id, name, age FROM users;";
    let rows = db_query(&pool, select_all_query, &[]).await?;

    for row in rows {
        let id: i32 = row.get("id");
        let name: String = row.get("name");
        let age: i32 = row.get("age");
        println!("User found -> ID: {}, Name: {}, Age: {}", id, name, age);
    }
    println!();


    // db_query_one()
    println!("--- Testing db_query_one() ---");
    
    let select_one_query = "SELECT name FROM users WHERE id = $1;";
    
    let row = db_query_one(&pool, select_one_query, &[&1]).await?;

    let name: String = row.get("name");
    println!("Exactly one user found: {}\n", name);


    // db_query_opt()
    println!("--- Testing db_query_opt() ---");
    let select_opt_query = "SELECT name FROM users WHERE id = $1;";
    
    let maybe_row = db_query_opt(&pool, select_opt_query, &[&999]).await?;

    match maybe_row {
        Some(row) => {
            let name: String = row.get("name");
            println!("User 999 exists: {}", name);
        }
        None => println!("User 999 does not exist (Safe fallback!)."),
    }

    Ok(())
}
```

# Web Socket Handling:
```rust
// Implement in config function

// Your app config
pub fn app_config(cfg: &mut ServiceConfig) {
  /* Configurations */
  cfg
  // Implemented web socket
  .configure(impl_ws)
  // Add the routes
  .service(ws)
  .service(message_group);
}

// Here's Example usage

// The WebSocket Route
#[get("/ws/{user_id}")]
async fn ws(
    req: Rqs,
    body: Payload,
    manager: AppData<WsManager>,
    path: Path<String>
) -> RltRsp {
    let user_id = path.into_inner();
    // Update connection to web socket.
    let (response, session, mut msg_stream) = actix_ws::handle(&req, body)?;

    // This handles the sending logic automatically.
    let guard = manager.register(&user_id, session);

    // Spawn the receive loop. YOU have full control here.
    rt::spawn(async move {
        // Move the guard into the task so it lives as long as the connection
        let _keep_alive = guard;

        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Text(text) => {
                println!("{user_id}: {text}");
                }
                Message::Close(_) => {
                // Break the loop on close. Dropping `_keep_alive` removes the user.
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(response)
}


// Sending to Users (from another route)
#[get("/message")]
async fn message_group(manager: AppData<WsManager>) -> Rsp {
    let target_users = vec!["user123", "user456", "admin99"];
    // send to multiple user_id
    manager.send_to_users(
        &target_users,
        AppMessage::Notification { 
            title: "Group Update".to_string(), 
            body: "Meeting starts in 5 minutes".to_string() 
        }
    );
    // send to one user_id
    manager.send_to_user(
    "user123",
    AppMessage::DirectMessage{
        from: "user123".to_string(),
        content: "Hi!".to_string(),
    }
    );
    // Broadcast / Send to every connected users
    manager.broadcast(AppMessage::SystemAlert("System alert!".to_string()));
    
    // Done
    http_ok("Ok!")
}
```

# Another functions:
```rust
// Http Ok, send string body
async fn some_fn() -> Rsp {
  http_ok("Ok!")
}

// Http Bad, send string body
async fn some_fn() -> Rsp {
  http_bad("Error!")
}

// Http Ok, send json body
async fn some_fn() -> Rsp {
  http_ok_json(json_data)
}

// Http Bad, send json body
async fn some_fn() -> Rsp {
  http_bad_json(json_data)
}

// Send file, also capable to send large file in streaming, automatically managed.
async fn some_fn() -> Rsp {
  send_file("./file/path/")
}

// Send string body
async fn some_fn() -> Rsp {
  send_str("Hello!")
}

// Send json body
async fn some_fn() -> Rsp {
  // We'll know, json sending ways.
  
  // But now, just understand, it'll send json data.
  send_json(json)
}
```

# JSON sending ways:
```rust
// Way 1
#[derive(Serialize)]
struct User {
    id: u32,
    name: String,
}

async fn user() -> Rsp {
    let data = User {
        id: 1,
        name: "Sayed".to_string(),
    };

    send_json(data)
}


// Way 2
send_json(vec![1, 2, 3])

// Way 3
let json = json!({
    "success": true,
    "message": "Logged in"
};
http_bad_json(json)


// Way 4
use std::collections::HashMap;

let mut map = HashMap::new();
map.insert("name", "Sayed");

http_ok_json(map)


/* Same result: {
  "id": 1,
  "name": "Sayed"
}
*/


```

# Modules:
```rust
// These all are available.

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
```


# A clean look:
```rust
// Code from clean.rs, that happens internally.
use actix_web::{
  HttpResponse,
  Error,
  HttpRequest
};
// A clean shorthand actix result
type Rlt = actix_web::Result<()>;
// A clean shorthand response
type Rsp = HttpResponse;
// A clean shorthand for a standard Actix Result
type RltRsp = Result<HttpResponse, Error>;
// A clean shorthand request
type Rqs = HttpRequest;
```