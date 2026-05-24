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

// Email configuration
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub from_name: String,
    pub from_email: String,
}

impl EmailService {
    pub fn new(email_config: EmailConfig) -> Result<Self, lettre::transport::smtp::Error> {
        let creds = Credentials::new(
            email_config.smtp_user.to_owned(),
            email_config.smtp_pass.to_owned(),
        );

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(email_config.smtp_host)?
            .port(email_config.smtp_port)
            .credentials(creds)
            .build();

        Ok(Self {
            mailer: Arc::new(mailer),
            from: Mailbox::new(
                Some(email_config.from_name.to_owned()),
                email_config.from_email.parse().unwrap(),
            ),
        })
    }

    pub async fn send_email(
        &self,
        data: EmailData<'_>,
    ) -> Result<(), lettre::transport::smtp::Error> {
        let email = Message::builder()
            .from(self.from.clone())
            .to(data.to.parse().unwrap())
            .subject(data.subject)
            .body(data.body)
            .unwrap();

        self.mailer.send(email).await?;

        Ok(())
    }
}
