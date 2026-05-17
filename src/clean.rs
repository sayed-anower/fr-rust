use actix_web::{Error, HttpRequest, HttpResponse};
// A clean shorthand actix result
pub type Rlt = actix_web::Result<()>;
// A clean shorthand response
pub type Rsp = HttpResponse;
// A clean shorthand for a standard Actix Result
pub type RltRsp = Result<HttpResponse, Error>;
// A clean shorthand request
pub type Rqs = HttpRequest;
