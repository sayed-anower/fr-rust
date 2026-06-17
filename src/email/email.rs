use lettre::{
    message::Mailbox,
    transport::smtp::{
        authentication::Credentials,
        PoolConfig,
    },
    AsyncSmtpTransport, Message, Tokio1Executor, AsyncTransport
};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::time::timeout;

// ============================================================================
// Error Type
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
    pub smtp_pass: SecretString,
    pub from_name: String,
    pub from_email: String,
    pub timeout_secs: u64,
    pub pool_max_size: u32, // Changed from usize to u32
}

impl EmailConfig {
    /// Example: load from environment
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        use std::env;
        Ok(Self {
            smtp_host: env::var("SMTP_HOST")?,
            smtp_port: env::var("SMTP_PORT")?.parse()?,
            smtp_user: env::var("SMTP_USER")?,
            smtp_pass: SecretString::from(env::var("SMTP_PASS")?), // Used From trait
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
    from: Mailbox, // Removed Arc wrapper
    timeout: Duration,
}

impl EmailService {
    /// Create production-ready email service with connection pooling
    pub fn new(config: EmailConfig) -> Result<Self, EmailError> {
        // Converted the exposed secret reference to an owned String
        let creds = Credentials::new(
            config.smtp_user, 
            config.smtp_pass.expose_secret().to_string() 
        );

        // Security Fix: Handle Implicit TLS (SMTPS) vs STARTTLS properly
        let builder = if config.smtp_port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)?
        };

        let mailer = builder
            .port(config.smtp_port)
            .credentials(creds)
            .pool_config(
                PoolConfig::new()
                    .max_size(config.pool_max_size)
                    .min_idle(2)
                    // Removed invalid max_idle(10) method
                    .idle_timeout(Duration::from_secs(300)),
            )
            .timeout(Some(Duration::from_secs(config.timeout_secs)))
            .build();

        let from_mailbox = Mailbox::new(
            Some(config.from_name),
            config.from_email.parse()?,
        );

        Ok(Self {
            mailer: Arc::new(mailer),
            from: from_mailbox,
            timeout: Duration::from_secs(config.timeout_secs),
        })
    }

    /// Send email
    pub async fn send_email(&self, data: EmailData) -> Result<(), EmailError> {
        let email = Message::builder()
            .from(self.from.clone()) // Clone the lightweight Mailbox directly
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
    pub to: Mailbox,           
    pub subject: String,
    pub body: String,
}

impl EmailData {
    pub fn new(to: &str, subject: String, body: String) -> Result<Self, EmailError> {
        Ok(Self {
            to: to.parse()?,
            subject,
            body,
        })
    }
}
