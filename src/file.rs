use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use serde::{de::DeserializeOwned, Serialize};
use tokio::{fs::OpenOptions, io::AsyncWriteExt, task::JoinHandle};
use tracing::{debug, error};

use crate::{load_object, models::{CreatedObject, MinimalObject, ObjectType}, project::Project, prtb::ProjectRoleTemplateBinding, rt::RoleTemplate, serialize_object};

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

impl FileFormat {
    /// Serialize a value into a string given the specified file format.
    ///
    /// # Errors
    ///
    /// This function will return an error if the serialization fails.
    ///
    /// # Examples
    ///
    /// 
    pub fn serialize<T: Serialize>(&self, value: &T) -> Result<String> {
        match self {
            FileFormat::Yaml => serde_yaml::to_string(value).map_err(|e| e.into()),
            FileFormat::Json => serde_json::to_string_pretty(value).map_err(|e| e.into()),
            FileFormat::Toml => toml::to_string(value).map_err(|e| e.into()),
        }
    }
    
    /// Deserialize a value from a string given the specified file format.
    ///
    /// # Errors
    ///
    /// This function will return an error if the deserialization fails.
    ///
    /// # Examples
    ///
    /// 
    pub fn deserialize<T: DeserializeOwned>(&self, data: &str) -> Result<T> {
        match self {
            FileFormat::Yaml => serde_yaml::from_str(data).map_err(|e| e.into()),
            FileFormat::Json => serde_json::from_str(data).map_err(|e| e.into()),
            FileFormat::Toml => toml::from_str(data).map_err(|e| e.into()),
        }
    }
}


/// Reads an object from a file path and returns a MinimalObject of the specified type
///
/// # Arguments
/// * `object_type` - The type of object to read from the file
/// * `path` - The path of the file to read from
///
/// # Returns
/// * `Result<MinimalObject, ConversionError>` - The minimal object loaded from the file
///
pub async fn get_minimal_object_from_path(object_type: ObjectType, path: &Path) -> Result<MinimalObject> {
    let file_format = file_format_from_path(path);
    match object_type {
        ObjectType::Project => {
            let object: Project = load_object(path, &file_format).await.unwrap();
            MinimalObject::try_from(object)
        },
        ObjectType::RoleTemplate => {
            let object: RoleTemplate = load_object(path, &file_format).await.unwrap();
            MinimalObject::try_from(object)
        },
        ObjectType::ProjectRoleTemplateBinding => {
            let object: ProjectRoleTemplateBinding = load_object(path, &file_format).await.unwrap();
            MinimalObject::try_from(object)
        }
        ObjectType::Cluster => {
            bail!("Mininal Object for Cluster unimplemented")
        }
    }
}


// Function to convert the retrieved contents to MinimalObject
pub async fn get_minimal_object_from_contents(
    object_type: ObjectType,
    contents: &str,
    file_format: &FileFormat,
) -> Result<MinimalObject> {
    match object_type {
        ObjectType::Project => {
            debug!("Deserializing Project: {:#?}", contents);
            // let object: Project = deserialize_object(contents, file_format)?;
            let object: Project = file_format.deserialize(contents)?;
            MinimalObject::try_from(object)
        },
        ObjectType::RoleTemplate => {
            debug!("Deserializing RT: {:#?}", contents);
            let object: RoleTemplate = file_format.deserialize(contents)?;
            MinimalObject::try_from(object)
        },
        ObjectType::ProjectRoleTemplateBinding => {
            debug!("Deserializing PRTB: {:#?}", contents);
            let object: ProjectRoleTemplateBinding = file_format.deserialize(contents)?;
            MinimalObject::try_from(object)
        },
        ObjectType::Cluster => {
            bail!("Minimal Object for Cluster unimplemented")
        }
    }
}


/// Writes back successfully created objects to their respective files
///
/// # Arguments
/// * `successes` - A vector of tuples containing the file path and created object
/// * `file_format` - The format to use for serialization
///
/// # Returns
/// A Result with a vector of file paths that were successfully written or error information
pub async fn write_back_objects(
    successes: Vec<(PathBuf, CreatedObject)>,
    file_format: FileFormat,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut handles: Vec<JoinHandle<anyhow::Result<PathBuf>>> = Vec::new();
    let mut results = Vec::new();

    // Spawn tasks to write back objects
    for (file_path, created_object) in successes {
        let format = file_format;
        handles.push(tokio::spawn(async move {
            match created_object {
                CreatedObject::ProjectRoleTemplateBinding(created) => {
                    debug!("Writing PRTB: {:#?}", created);
                    let convert = ProjectRoleTemplateBinding::try_from(created)?;
                    write_object_to_file(&file_path, &format, &convert).await?;
                    Ok(file_path)
                }
                CreatedObject::Project(created) => {
                    debug!("Writing Project: {:#?}", created);
                    let convert = Project::try_from(created)?;
                    write_object_to_file(&file_path, &format, &convert).await?;
                    Ok(file_path)
                }
                CreatedObject::RoleTemplate(created) => {
                    debug!("Writing Role Template: {:#?}", created);
                    let convert = RoleTemplate::try_from(created)?;
                    write_object_to_file(&file_path, &format, &convert).await?;
                    Ok(file_path)
                }
                _ => {
                    anyhow::bail!("Writing back object type not implemented")
                }
            }
        }));
    }

    // Wait for all tasks to complete and collect results
    for handle in handles {
        match handle.await {
            Ok(result) => match result {
                Ok(path) => {
                    debug!("Successfully wrote to file: {}", path.display());
                    results.push(path);
                }
                Err(e) => {
                    error!("Error writing object: {}", e);
                }
            },
            Err(join_err) => {
                error!("Task panicked: {:?}", join_err);
            }
        }
    }

    Ok(results)
}


/// Get the file name for a specific object type
pub fn get_file_name_for_object(
    object_id: &str, 
    object_type: &ObjectType, 
    file_format: &FileFormat
) -> String {
    let extension = file_extension_from_format(file_format);
    match object_type {
        ObjectType::Project => format!("{}.project.{}", object_id, extension),
        ObjectType::ProjectRoleTemplateBinding => format!("{}.prtb.{}", object_id, extension),
        ObjectType::RoleTemplate => format!("{}.rt.{}", object_id, extension),
        ObjectType::Cluster => format!("{}.cluster.{}", object_id, extension),
        // _ => format!("{}.{}", object_id, extension),
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
        "yaml" => FileFormat::Yaml,
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
        FileFormat::Yaml => "yaml".to_string(),
        FileFormat::Json => "json".to_string(),
        FileFormat::Toml => "toml".to_string(),
    }
}

pub fn file_format(file_format: &str) -> FileFormat {
    match file_format {
        "yml" => FileFormat::Yaml,
        "yaml" => FileFormat::Yaml,
        "json" => FileFormat::Json,
        "toml" => FileFormat::Toml,
        _ => FileFormat::Json,
    }
}
