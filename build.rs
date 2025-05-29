use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let client_name = "sheperd";
    let client_version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".into());

    let contents = format!(
        r#"pub const CLIENT_NAME: &str = "{}";
pub const CLIENT_VERSION: &str = "{}";
pub const FULL_CLIENT_ID: &str = "{}";"#,
        client_name,
        client_version,
        format!("{}/{}", client_name, client_version)
    );

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("client_info.rs");
    fs::write(dest_path, contents).unwrap();
}
