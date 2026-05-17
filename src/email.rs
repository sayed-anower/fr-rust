use lettre::{
    Message, SmtpTransport, Transport, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use std::error::Error;

pub struct EmailConfig<'a> {
    pub smtp_host: &'a str,
    pub smtp_port: u16,
    pub smtp_user: &'a str,
    pub smtp_pass: &'a str,
    pub from_name: &'a str,
    pub from_email: &'a str,
}

pub struct EmailData<'a> {
    pub to: &'a str,
    pub subject: &'a str,
    pub body: String, // Kept as String as body content is usually dynamic
}

pub fn send_email(config: EmailConfig, data: EmailData) -> Result<(), Box<dyn Error>> {
    // Build email - Using references avoids unnecessary cloning
    let email = Message::builder()
        .from(Mailbox::new(
            Some(config.from_name.to_string()),
            config.from_email.parse()?,
        ))
        .to(data.to.parse()?)
        .subject(data.subject)
        .body(data.body)?;

    // SMTP credentials
    let creds = Credentials::new(config.smtp_user.to_string(), config.smtp_pass.to_string());

    // SMTP client - Relay is built using the provided host and port
    let mailer = SmtpTransport::starttls_relay(config.smtp_host)?
        .port(config.smtp_port)
        .credentials(creds)
        .build();

    // Send email
    mailer.send(&email)?;

    Ok(())
}
