use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid value for field '{field}': {reason}")]
    InvalidValue {
        field: &'static str,
        reason: String,
    },

    #[error("Failed to convert metadata: {0}")]
    MetadataError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}






