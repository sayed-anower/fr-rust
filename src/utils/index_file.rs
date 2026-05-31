use crate::prelude::*;

#[get("/")]
pub async fn index_file() -> FileRlt {
    send_file("./static/index.html").await
}