use lettre::{
    AsyncSmtpTransport,
    AsyncTransport,
    Message,
    Tokio1Executor,
    message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use std::sync::Arc;

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
    // Fixed: Changed Result error type to a box/generic error to handle both SMTP and Address parsing errors smoothly
    pub fn new(email_config: EmailConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // OPTIMIZATION: Move strings instead of calling .to_owned()
        let creds = Credentials::new(email_config.smtp_user, email_config.smtp_pass);

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&email_config.smtp_host)?
            .port(email_config.smtp_port)
            .credentials(creds)
            .build();

        let from_mailbox = Mailbox::new(
            Some(email_config.from_name),
            email_config.from_email.parse()?, // Safe error handling instead of .unwrap()
        );

        Ok(Self {
            mailer: Arc::new(mailer),
            from: from_mailbox,
        })
    }

    // OPTIMIZATION: Pass data by reference to allow re-use and prevent unnecessary memory shifts
    pub async fn send_email(&self, data: &EmailData) -> Result<(), Box<dyn std::error::Error>> {
        // Safe parsing: returns an error instead of crashing if the recipient email is malformed
        let to_address = data.to.parse()?; 

        let email = Message::builder()
            .from(self.from.clone())
            .to(to_address)
            .subject(&data.subject)
            .body(data.body.clone())?; // Lettre needs to consume the body, so we clone here if passed by ref

        self.mailer.send(email).await?;

        Ok(())
    }
}