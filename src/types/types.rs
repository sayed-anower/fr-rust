use actix_web::{Error, HttpRequest, HttpResponse};
use actix_files::NamedFile;

pub type Rlt = Result<(), Error>;

pub type Rsp = HttpResponse;

pub type RltRsp = Result<HttpResponse, Error>;

pub type Rqs = HttpRequest;

pub type MainRlt = std::io::Result<()>;

pub type FileRlt = Result<NamedFile, Error>;
