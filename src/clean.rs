use actix_web::{Error, HttpRequest, HttpResponse};
// A clean shorthand actix result
type Rlt = actix_web::Result<()>;
// A clean shorthand response
type Rsp = HttpResponse;
// A clean shorthand for a standard Actix Result
type RltRsp = Result<HttpResponse, Error>;
// A clean shorthand request
type Rqs = HttpRequest;
