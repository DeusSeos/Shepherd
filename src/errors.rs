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