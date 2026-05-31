use crate::prelude::{
    ServiceConfig
};

// App Configuration
pub fn app_config(cfg: &mut ServiceConfig) {
    cfg.service(index_file);
}
