use std::sync::Arc;

use rancher_client::apis::configuration::{ApiKey, Configuration};
use reqwest_middleware::ClientBuilder;

fn rancher_config_init(endpoint_url: &str, token: &str) -> Configuration {
    let mut config = Configuration::new();
    config.base_path = endpoint_url.to_string();

    config.api_key = Some(ApiKey {
        prefix: Some("Bearer".to_string()),
        key: token.to_string(),
    });
    config
}




pub struct ShepherdClient {
    pub config: Arc<Configuration>,
}


impl ShepherdClient {
    pub fn new(endpoint_url: &str, token: &str, allow_insecure: bool) -> Self {
        let mut config = rancher_config_init(endpoint_url, token);

        if allow_insecure {
            // modify the configuration client to allow self-signed certificates, TODO: Remove this when we have proper certificate handling
            let client = reqwest::Client::builder()
                .danger_accept_invalid_certs(allow_insecure)
                .build()
                .unwrap();
            config.client = ClientBuilder::new(client).build();
        }

        Self {
            config: Arc::new(config),
        }
    }

}