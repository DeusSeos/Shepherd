use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use tokio::{fs::OpenOptions, io::AsyncWriteExt, task::JoinHandle};
use tracing::{debug, error};
use std::error::Error;

use crate::{deserialize_object, load_object, models::{ConversionError, CreatedObject, MinimalObject, ObjectType}, project::Project, prtb::ProjectRoleTemplateBinding, rt::RoleTemplate, serialize_object};

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


/// Reads an object from a file path and returns a MinimalObject of the specified type
///
/// # Arguments
/// * `object_type` - The type of object to read from the file
/// * `path` - The path of the file to read from
///
/// # Returns
/// * `Result<MinimalObject, ConversionError>` - The minimal object loaded from the file
///
pub async fn get_minimal_object_from_path(object_type: ObjectType, path: &Path) -> Result<MinimalObject, ConversionError> {
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
            Err(ConversionError::Other("Mininal Object for Cluster unimplemented".into()))
        }
    }
}


// Function to convert the retrieved contents to MinimalObject
pub async fn get_minimal_object_from_contents(
    object_type: ObjectType,
    contents: &str,
    file_format: &FileFormat,
) -> Result<MinimalObject, ConversionError> {
    match object_type {
        ObjectType::Project => {
            debug!("Deserializing Project: {:#?}", contents);
            let object: Project = deserialize_object(contents, file_format)?;
            MinimalObject::try_from(object)
        },
        ObjectType::RoleTemplate => {
            debug!("Deserializing RT: {:#?}", contents);
            let object: RoleTemplate = deserialize_object(contents, file_format)?;
            MinimalObject::try_from(object)
        },
        ObjectType::ProjectRoleTemplateBinding => {
            debug!("Deserializing PRTB: {:#?}", contents);
            let object: ProjectRoleTemplateBinding = deserialize_object(contents, file_format)?;
            MinimalObject::try_from(object)
        },
        ObjectType::Cluster => {
            Err(ConversionError::Other("Minimal Object for Cluster unimplemented".into()))
        }
    }
}


// Spawn tasks to write back the successfully created objects
// TODO: add error handling and return results
pub async fn write_back_objects(
    successes: Vec<(PathBuf, CreatedObject)>,
    file_format: FileFormat,
)
    {
    let mut handles: Vec<JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>> = Vec::new();
    for (file_path, created_object) in successes {
        let handle = tokio::spawn(async move {
            match created_object {
                CreatedObject::ProjectRoleTemplateBinding(created) => {
                    debug!("Writing PRTB: {:#?}", created);
                    let convert = ProjectRoleTemplateBinding::try_from(created)?;
                    write_object_to_file(&file_path, &file_format, &convert).await.unwrap_or_else(
                        |err| {
                            error!("Error writing PRTB: {:#?}", err);
                    });
                }
                CreatedObject::Project(created) => {
                    debug!("Writing Project: {:#?}", created);
                    let convert = Project::try_from(created)?;
                    write_object_to_file(&file_path, &file_format, &convert).await.unwrap_or_else(
                        |err| {
                            error!("Error writing Project: {:#?}", err);
                        },
                    );
                }
                CreatedObject::RoleTemplate(created) => {
                    debug!("Writing Role Template: {:#?}", created);
                    let convert = RoleTemplate::try_from(created)?;
                    write_object_to_file(&file_path, &file_format, &convert).await.unwrap_or_else(
                        |err| {
                            error!("Error writing Role Template: {:#?}", err);
                        },
                    );
                }
            }
            Ok(())
        });
        handles.push(handle);
    }

    for handle in handles {
        match handle.await {
            Err(join_err) => eprintln!("Task panicked: {:?}", join_err),
            Ok(Err(err)) => eprintln!("Task error: {:?}", err),
            Ok(Ok(_)) => {}
        }
    }

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
