// This file will contain all the functions that will be used to interact and extract from the Rancher API
pub mod cluster;
pub mod project;
pub mod rt;

use rancher_client::apis::configuration::{ApiKey, Configuration};


pub fn rancher_config_init(host: &str, token: &str) -> Configuration {
    let mut config = Configuration::new();
    config.base_path = host.to_string();

    config.api_key = Some(ApiKey {
        prefix: Some("Bearer".to_string()),
        key: token.to_string(),
    });

    config
}



