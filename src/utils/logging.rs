use tracing::error;


pub fn log_api_error(context: &str, error: &impl std::fmt::Debug) {
    // For anyhow::Error, this will include the entire error chain
    error!(context = context, error = ?error, "API operation failed");
}


pub fn generate_operation_id() -> String {
    let id: u64 = fastrand::u64(..);
    format!("{:x}", id)
}