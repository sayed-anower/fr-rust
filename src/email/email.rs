use lettre::{
    message::Mailbox,
    transport::smtp::{authentication::Credentials, PoolConfig},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use std::sync::Arc;
use thiserror::Error;

/// Unified error type for email parsing, message building, and SMTP transport.
#[derive(Debug, Error)]
pub enum EmailError {
    #[error("Invalid email address format: {0}")]
    Address(#[from] lettre::address::AddressError),

    #[error("Failed to construct email message: {0}")]
    Message(#[from] lettre::error::Error),

    #[error("SMTP transport error: {0}")]
    Transport(#[from] lettre::transport::smtp::Error),
}

#[derive(Clone)]
pub struct EmailService {
    mailer: Arc<AsyncSmtpTransport<Tokio1Executor>>,
    from: Mailbox,
}

pub struct EmailData {
    pub to: String,
    pub subject: String,
    pub body: String,
}

pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub from_name: String,
    pub from_email: String,
}

impl EmailService {
    /// Initializes the EmailService with connection pooling enabled.
    pub fn new(config: EmailConfig) -> Result<Self, EmailError> {
        let creds = Credentials::new(config.smtp_user, config.smtp_pass);

        // Build the mailer with a connection pool to reuse SMTP connections
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)?
            .port(config.smtp_port)
            .credentials(creds)
            .pool_config(PoolConfig::new().max_size(20)) // Adjust pool size based on traffic
            .build();

        let from_mailbox = Mailbox::new(
            Some(config.from_name),
            config.from_email.parse()?,
        );

        Ok(Self {
            mailer: Arc::new(mailer),
            from: from_mailbox,
        })
    }

    /// Sends an email. Takes ownership of `EmailData` to avoid unnecessary cloning.
    pub async fn send_email(&self, data: EmailData) -> Result<(), EmailError> {
        let to_address = data.to.parse()?; 

        // Message body consumes the string, so taking `data` by value prevents an extra allocation
        let email = Message::builder()
            .from(self.from.clone())
            .to(to_address)
            .subject(data.subject)
            .body(data.body)?; 

        self.mailer.send(email).await?;

        Ok(())
    }
}
