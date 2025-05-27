use crate::models::ConversionError;


#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("API error: {0}")]
    ApiError(#[from] rancher_client::apis::Error<()>),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Object conversion error: {0}")]
    ConversionError(#[from] ConversionError),
    
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn configuration_error(msg: impl Into<String>) -> Self {
        AppError::Other(format!("Configuration error: {}", msg.into()))
    }
}




pub fn handle_result_collection<T, E>(results: Vec<Result<T, E>>) -> (Vec<T>, Vec<E>) 
where 
    E: std::fmt::Debug
{
    let mut successes = Vec::new();
    let mut errors = Vec::new();
    
    for result in results {
        match result {
            Ok(value) => successes.push(value),
            Err(err) => errors.push(err),
        }
    }
    
    (successes, errors)
}