use crate::*;
use fr_rust::prelude::*;

// App Configuration
pub fn app_config(cfg: &mut ServiceConfig) {
    // Configured
    cfg
    // Configured shared states
       .configure(shared_state)
    // Configured routes
       .service(index_file);
}
