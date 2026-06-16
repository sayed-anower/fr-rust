use lettre::{
    message::Mailbox,
    transport::smtp::{
        authentication::Credentials,
        PoolConfig, SmtpTransportBuilder,
    },
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::time::timeout;

// ============================================================================
// Error Type (Production-friendly)
// ============================================================================

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("Invalid email address: {0}")]
    Address(#[from] lettre::address::AddressError),

    #[error("Failed to build email message")]
    Message(#[from] lettre::error::Error),

    #[error("SMTP transport error")]
    Transport(#[from] lettre::transport::smtp::Error),

    #[error("Operation timed out")]
    Timeout,
}

// ============================================================================
// Configuration
// ============================================================================

#[derive(Clone)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: SecretString,        // ← Use secrecy crate
    pub from_name: String,
    pub from_email: String,
    pub timeout_secs: u64,              // default 15
    pub pool_max_size: usize,
}

impl EmailConfig {
    /// Example: load from environment
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        use std::env;
        Ok(Self {
            smtp_host: env::var("SMTP_HOST")?,
            smtp_port: env::var("SMTP_PORT")?.parse()?,
            smtp_user: env::var("SMTP_USER")?,
            smtp_pass: SecretString::new(env::var("SMTP_PASS")?),
            from_name: env::var("SMTP_FROM_NAME")?,
            from_email: env::var("SMTP_FROM_EMAIL")?,
            timeout_secs: env::var("SMTP_TIMEOUT_SECS")
                .unwrap_or_else(|_| "15".into())
                .parse()?,
            pool_max_size: env::var("SMTP_POOL_SIZE")
                .unwrap_or_else(|_| "20".into())
                .parse()?,
        })
    }
}

// ============================================================================
// Email Service
// ============================================================================

#[derive(Clone)]
pub struct EmailService {
    mailer: Arc<AsyncSmtpTransport<Tokio1Executor>>,
    from: Arc<Mailbox>,
    timeout: Duration,
}

impl EmailService {
    /// Create production-ready email service with connection pooling
    pub fn new(config: EmailConfig) -> Result<Self, EmailError> {
        let creds = Credentials::new(config.smtp_user, config.smtp_pass.expose_secret().clone());

        let mut builder: SmtpTransportBuilder = if config.smtp_port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)?
        };

        let mailer = builder
            .port(config.smtp_port)
            .credentials(creds)
            .pool_config(
                PoolConfig::new()
                    .max_size(config.pool_max_size)
                    .min_idle(2)
                    .max_idle(10)
                    .idle_timeout(Duration::from_secs(300)),
            )
            .timeout(Duration::from_secs(config.timeout_secs))
            .build();

        let from_mailbox = Mailbox::new(
            Some(config.from_name),
            config.from_email.parse()?,
        );

        Ok(Self {
            mailer: Arc::new(mailer),
            from: Arc::new(from_mailbox),
            timeout: Duration::from_secs(config.timeout_secs),
        })
    }

    /// Send email (high performance path)
    pub async fn send_email(&self, data: EmailData) -> Result<(), EmailError> {
        let email = Message::builder()
            .from(Arc::clone(&self.from))
            .to(data.to)
            .subject(data.subject)
            .body(data.body)?;

        // Timeout protection
        match timeout(self.timeout, self.mailer.send(email)).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err(EmailError::Timeout),
        }
    }
}

// ============================================================================
// Email Data (Performance optimized)
// ============================================================================

#[derive(Debug)]
pub struct EmailData {
    pub to: Mailbox,           // Parse once, outside hot path
    pub subject: String,
    pub body: String,
}

impl EmailData {
    /// Helper to create from strings
    pub fn new(to: &str, subject: String, body: String) -> Result<Self, EmailError> {
        Ok(Self {
            to: to.parse()?,
            subject,
            body,
        })
    }
}

// ============================================================================
// Example Usage (main.rs)
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load config from environment (recommended)
    let config = EmailConfig::from_env()
        .expect("Failed to load SMTP config from environment");

    let email_service = EmailService::new(config)?;

    // Example 1: Simple send
    let data = EmailData::new(
        "user@example.com",
        "Welcome to Our Service!".to_string(),
        "Hello,\n\nThank you for joining us.".to_string(),
    )?;

    email_service.send_email(data).await?;
    println!("Email sent successfully!");

    // Example 2: Bulk sending with concurrency control
    let mut tasks = vec![];
    for i in 0..5 {
        let service = email_service.clone();
        let data = EmailData::new(
            &format!("user{}@example.com", i),
            format!("Test Email #{}", i),
            "This is a test.".to_string(),
        )?;

        tasks.push(tokio::spawn(async move {
            service.send_email(data).await
        }));
    }

    // Wait for all
    for task in tasks {
        if let Err(e) = task.await? {
            eprintln!("Failed to send one email: {}", e);
        }
    }

    Ok(())
}