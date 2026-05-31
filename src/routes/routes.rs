use crate::prelude::*;

#[post("/login")]
async fn login(
  
) -> Rsp {
  
}
#[post("/signup")]
async fn login(
  db: web::Data<DbPool>,
  redis_pool: web::Data<RedisManager>,
  otp: web::Data<OtpService>,
  linkv: web::Data<LinkV>,
  email: web::Data<EmailService>,
  crypto: web::Data<CryptoService>,
  
) -> Rsp {
  
}