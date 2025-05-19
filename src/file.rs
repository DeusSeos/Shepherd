use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use crate::serialize_object;


#[derive(Clone, Copy)]
pub enum FileFormat {
    Yaml,
    Json,
    Toml,
}

// to string for FileFormat
impl std::fmt::Display for FileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", file_extension_from_format(self))
        }
    }


/// Generic function to write any type of object to a file in the given path (overwrites file content)
/// `file_path` is the path to the directory where the file should be written
/// `file_format` is the format of the file to write (yaml, json, or toml)
///
/// Returns a Result
pub async fn write_object_to_file<T>(
    file_path: &PathBuf,
    file_format: &FileFormat,
    object: &T,
) -> Result<()>
where
    T: serde::Serialize + Send + 'static,
{
    let serialized = serialize_object(object, file_format)?;
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(file_path)
        .await?;
    file.write_all(serialized.as_bytes())
        .await
        .context("Failed to write object to file")
}



pub fn file_format_from_extension(extension: &str) -> FileFormat {
    match extension {
        "yml" => FileFormat::Yaml,
        "json" => FileFormat::Json,
        "toml" => FileFormat::Toml,
        _ => FileFormat::Json,
    }
}

pub fn file_format_from_path(path: &Path) -> FileFormat {
    match path.extension() {
        Some(ext) => file_format_from_extension(ext.to_str().unwrap()),
        None => FileFormat::Json,
    }
}

pub fn file_extension_from_format(file_format: &FileFormat) -> String {
    match file_format {
        FileFormat::Yaml => "yml".to_string(),
        FileFormat::Json => "json".to_string(),
        FileFormat::Toml => "toml".to_string(),
    }
}

pub fn file_format(file_format: &str) -> FileFormat {
    match file_format {
        "yaml" => FileFormat::Yaml,
        "json" => FileFormat::Json,
        "toml" => FileFormat::Toml,
        _ => FileFormat::Json,
    }
}