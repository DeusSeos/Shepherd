use anyhow::Result;
use rancher_client::apis::configuration::Configuration;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::models::{CreatedObject, MinimalObject, ObjectType, ResourceVersionMatch};
use crate::utils::logging::log_api_error;

pub trait RancherResource: Sized + Clone + DeserializeOwned + Serialize {
    type ApiType: Clone + DeserializeOwned + Serialize;
    
    // Resource metadata
    fn resource_type() -> ObjectType;
    fn exclude_paths() -> &'static [&'static str];
    
    // Conversion methods
    fn try_from_api(value: Self::ApiType) -> Result<Self>;
    fn try_into_api(self) -> Result<Self::ApiType>;
    
    // Identity methods
    fn id(&self) -> Option<String>;
    fn namespace(&self) -> Option<String>;
    fn resource_version(&self) -> Option<String>;
    
    // Create a minimal object representation
    fn to_minimal_object(&self) -> MinimalObject {
        MinimalObject {
            object_id: self.id(),
            resource_version_match: ResourceVersionMatch::Exact,
            resource_version: self.resource_version(),
            namespace: self.namespace(),
        }
    }
    
    // Common API operations with default implementations
    fn list(_config: &Configuration,_namespacee: Option<&str>) -> impl std::future::Future<Output = Result<Vec<Self::ApiType>>> + Send {async {
        // Default implementation would dispatch to the appropriate API call
        // based on resource_type()
        unimplemented!("List operation must be implemented by resource type")
    } }
    
    fn get(_config: &Configuration,_name: &str, _namespace: &str) -> impl std::future::Future<Output = Result<Self>> + Send {async {
        unimplemented!("Get operation must be implemented by resource type")
    } }
    
    fn create(&self, _config: &Configuration) -> impl std::future::Future<Output = Result<CreatedObject>> + Send {async {
        unimplemented!("Create operation must be implemented by resource type")
    } }
    
    fn update(&self, _config: &Configuration,_patch: Value) -> impl std::future::Future<Output = Result<CreatedObject>> + Send {async {
        unimplemented!("Update operation must be implemented by resource type")
    } }
    
    fn delete(_config: &Configuration, _name: &str, _namespace: &str) -> impl std::future::Future<Output = Result<CreatedObject>> + Send {async {
        unimplemented!("Delete operation must be implemented by resource type")
    } }
    
    // Helper for handling API errors
    fn handle_api_error<T: std::fmt::Debug>(result: Result<T>, operation: &str) -> Result<T> {
        match result {
            Ok(value) => Ok(value),
            Err(err) => {
                log_api_error(operation, &err);
                Err(anyhow::anyhow!("API error during {}: {}", operation, err))
            }
        }
    }
}