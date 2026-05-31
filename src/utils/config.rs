use crate::prelude::{
    ServiceConfig
};
use crate::utils::index_file::index_file;

// App Configuration
pub fn app_config(cfg: &mut ServiceConfig) {
    cfg.service(index_file);
}
