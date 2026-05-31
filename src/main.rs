use fr_rust::prelude::*;

#[actix_web::main]
async fn main() -> MainRlt {
    dotenv().ok();
    /* SHARED STATES */
    // DDoS Shield
    let ddos_shield = DdosShield::builder()
    // max requests per 1 sec
        .max_requests(5)
    // Per 1 sec
        .window_secs(1)
        .ban_duration_secs(20)
        .block_agent("malicious-bot")
        .allow_missing_ua(false)
        .build();
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
    let database_url = env_var("DATABASE_URL");
    let pool = DbPool::new(database_url);

    // Redis
    let redis_config = RedisConfig {
            url: env_var("REDIS_URL"),
            max_channels_per_pubsub: 2, 
            flush_interval_ms: 100, 
            pool_max_size: 3, 
    };
    let redis = RedisManager::new(redis_config).await.unwrap();
    // Crypto
    let key = env_var("AES_KEY");
    let key_bytes: &[u8; 32] = key.as_bytes().try_into().expect("AES_KEY must be exactly 32 bytes");
    let crypto_service = CryptoService::new(key_bytes).unwrap();
    // Otp verification
    let otp_config = OtpConfig {
        secret: env_var("KEY"),
        crypto: crypto_service.clone(),
        redis: redis.clone(),
        ttl_secs: 300 
    };
    let otp_service = OtpService::new(otp_config);
    // Link verification
    let linkv_config = LinkVConfig {
        secret: env_var("KEY"),
        crypto: crypto_service.clone(),
        redis: redis.clone(),
        ttl_secs: 300 
    };
    let linkv_service = LinkV::new(linkv_config);
    // Web Socket
    //let ws = WsService::new(redis.clone(), pool.clone()); // Web Socket has errors.
    /* IP & PORTS */
    let ip = env_var_or_default("IP", "0.0.0.0");
    let port = env_var_or_default("PORT", "8080");
    let address = format!("{}:{}", ip, port);
    /* START SERVER */
    println!("Starting server at http://{}", address);
    HttpServer::new(move || App::new()
    .app_data(AppData::new(email_service.clone()))
    .app_data(AppData::new(pool.clone()))
    .app_data(AppData::new(redis.clone()))
    .app_data(AppData::new(crypto_service.clone()))
    .app_data(AppData::new(otp_service.clone()))
    .app_data(AppData::new(linkv_service.clone()))
    // .app_data(AppData::new(ws.clone()))
    .configure(app_config)
    .wrap(ddos_shield.clone())
    ).bind(address)?.run().await
}
