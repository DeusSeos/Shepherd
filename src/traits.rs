use std::future::Future;

use rancher_client::apis::configuration::Configuration;

use anyhow::Result;

pub trait RancherObject: Sized {
    type ApiType;
    // TODO: Update the function arguments for the create, delete, update, and get functions (some may require more arguments for namespaces(cluster, or project))
    fn create(&self, config: &Configuration) -> impl Future<Output = Result<Self::ApiType>>;
    fn delete(id: &str, config: &Configuration) -> impl Future<Output = Result<Self::ApiType>>;
    fn update(id: &str, config: &Configuration) -> impl Future<Output = Result<Self::ApiType>>;
    fn get(id: &str, config: &Configuration) -> impl Future<Output = Result<Self::ApiType>>;
}