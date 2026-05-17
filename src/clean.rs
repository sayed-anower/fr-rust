use actix_web::{Error, HttpRequest, HttpResponse};
use actix_files::NamedFile;

// A clean shorthand actix result
pub type Rlt = actix_web::Result<()>;
// A clean shorthand response
pub type Rsp = HttpResponse;
// A clean shorthand for a standard Actix Result
pub type RltRsp = Result<HttpResponse, Error>;
// A clean shorthand request
pub type Rqs = HttpRequest;
// A clean shorthand version
pub type MainRlt = Result<(), std::io::Error>;	
pub type FileRlt = Result<NamedFile, Error>;
