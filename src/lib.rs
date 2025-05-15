// This file will contain all the functions that will be used to interact and extract from the Rancher API
pub mod cluster;
pub mod config;
pub mod diff;
pub mod file;
pub mod git;
pub mod project;
pub mod prtb;
pub mod rt;
pub mod update;

use file::{file_extension_from_format, FileFormat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::option::Option;
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, read_dir, read_to_string, write};

use cluster::Cluster;
use config::{ClusterConfig, RancherClusterConfig};
use project::{create_project, Project};
use prtb::{create_project_role_template_binding, ProjectRoleTemplateBinding};
use rt::{create_role_template, get_role_templates, RoleTemplate};

use rancher_client::apis::configuration::{ApiKey, Configuration};
use rancher_client::models::{
    IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding,
    IoCattleManagementv3RoleTemplate,
};

include!(concat!(env!("OUT_DIR"), "/client_info.rs"));

pub fn rancher_config_init(host: &str, token: &str) -> Configuration {
    let mut config = Configuration::new();
    config.base_path = host.to_string();

    config.api_key = Some(ApiKey {
        prefix: Some("Bearer".to_string()),
        key: token.to_string(),
    });
    config
}

// Local usage only, for testing purposes
// This function will be used to fetch the current configuration from the Rancher API
// it will do the following:
// 1. Get the current configuration from the Rancher API for clusters, projects, role templates and project role template bindings
// 2. Loop through the clusters and save them to a folder
// 3. Loop through the projects and save them to the correct cluster folder
// 4. Loop through the role templates and save them to the correct cluster folder
// 5. Loop through the project role template bindings and save them to the correct project folder

//
// the example folder structure will be as follows:


// /c-293x
// ├─ c-293x.(yaml/json/toml)
// ├─ /p-2a4i21
// │　├─ p-2a4i21.(yaml/json/toml)
// │　└─ prtb-29291.(yaml/json/toml)
// ├─ /p-8a2h12
// │　├─ p-8a2h12.(yaml/json/toml)
// │　└─ prtb-9nn91.(yaml/json/toml)
// └─ roles
// 　　├─ rt-92813.(yaml/json/toml)
// 　　└─ rt-92818.(yaml/json/toml)

// the file names will be the ID of the object
// the file extension will be the format of the file
// the folder names will be the ID of the object
// the folder will be created if it does not exist
// the function will return the path to the folderm

#[async_backtrace::framed]
pub async fn download_current_configuration(
    configuration: &Configuration,
    path: &Path,
    file_format: &FileFormat,
) -> Result<(), Box<dyn Error>> {
    let rancher_cluster = cluster::get_clusters(configuration)
        .await
        .map_err(|e| format!("Failed to get clusters: {e:?}"))?;

    let rancher_role_templates =
        get_role_templates(configuration, None, None, None, None, None, None)
            .await
            .map_err(|e| format!("Failed to get role templates: {e:?}"))?;

    let base_path = path.join(
        configuration
            .base_path
            .trim_end_matches('/')
            .replace("https://", "")
            .replace('/', "_"),
    );
    if !base_path.exists() {
        create_dir_all(&base_path)
            .await
            .map_err(|e| format!("Failed to create folder: {e:?}"))?;
    }

    let role_template_path = base_path.join("roles");
    if !role_template_path.exists() {
        create_dir_all(&role_template_path)
            .await
            .map_err(|e| format!("Failed to create folder: {e:?}"))?;
    }

    let role_templates: Vec<RoleTemplate> = rancher_role_templates
        .items
        .into_iter()
        .map(|item| {
            item.try_into()
                .map_err(|e| format!("Failed to convert role template: {e}"))
        })
        .collect::<Result<_, _>>()?;

    for role_template in &role_templates {
        let role_template_file = role_template_path.join(format!(
            "{}.{}",
            role_template.id,
            file_extension_from_format(file_format)
        ));
        write(
            &role_template_file,
            serialize_object(role_template, file_format)?,
        )
        .await
        .map_err(|e| format!("Failed to write file {:?}: {e}", role_template_file))?;
    }

    let clusters: Vec<Cluster> = rancher_cluster
        .items
        .into_iter()
        .map(|item| {
            item.try_into()
                .map_err(|e| format!("Failed to convert cluster: {e:?}"))
        })
        .collect::<Result<_, _>>()?;

    for cluster in &clusters {
        let cluster_path = base_path.join(&cluster.id);
        if !cluster_path.exists() {
            create_dir_all(&cluster_path)
                .await
                .map_err(|e| format!("Failed to create folder: {e:?}"))?;
        }

        let cluster_file = cluster_path.join(format!(
            "{}.{}",
            cluster.id,
            file_extension_from_format(file_format)
        ));
        write(&cluster_file, serialize_object(cluster, file_format)?)
            .await
            .map_err(|e| format!("Failed to write file: {e:?}"))?;

        let rancher_projects = project::get_projects(
            configuration,
            &cluster.id,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| format!("Failed to get projects: {e:?}"))?;

        let projects: Vec<Project> = rancher_projects
            .items
            .into_iter()
            .map(|item| {
                item.try_into()
                    .map_err(|e| format!("Failed to convert project: {e:?}"))
            })
            .collect::<Result<_, _>>()?;

        for project in &projects {
            let project_path = cluster_path.join(&project.display_name);
            if !project_path.exists() {
                create_dir_all(&project_path)
                    .await
                    .map_err(|e| format!("Failed to create folder: {e:?}"))?;
            }

            let project_file = project_path.join(format!(
                "{}.{}",
                project.display_name,
                file_extension_from_format(file_format)
            ));
            write(&project_file, serialize_object(project, file_format)?)
                .await
                .map_err(|e| format!("Failed to write file: {e:?}"))?;

            let rancher_prtbs = prtb::get_namespaced_project_role_template_bindings(
                configuration,
                &project.id.as_ref().unwrap().to_string(),
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| format!("Failed to get PRTBs: {e:?}"))?;

            let prtbs: Vec<ProjectRoleTemplateBinding> = rancher_prtbs
                .items
                .into_iter()
                .map(|item| {
                    item.try_into()
                        .map_err(|e| format!("Failed to convert PRTB: {e:?}"))
                })
                .collect::<Result<_, _>>()?;

            for prtb in &prtbs {
                let prtb_file = project_path.join(format!(
                    "{}.{}",
                    prtb.id,
                    file_extension_from_format(file_format)
                ));
                write(&prtb_file, serialize_object(prtb, file_format)?)
                    .await
                    .map_err(|e| format!("Failed to write file: {e:?}"))?;
            }
        }
    }

    Ok(())
}

#[async_backtrace::framed]
pub async fn load_configuration_from_rancher(
    configuration: &Configuration,
    cluster_id: &str,
) -> Result<RancherClusterConfig, Box<dyn Error>> {
    // Get the current configuration from the Rancher API
    let rancher_clusters = cluster::get_clusters(configuration)
        .await
        .map_err(|e| Box::<dyn Error>::from(format!("Failed to get clusters: {:?}", e)))?;
    let rancher_cluster = rancher_clusters
        .items
        .into_iter()
        .find(|cluster| {
            cluster.metadata.as_ref().and_then(|m| m.name.as_deref()) == Some(cluster_id)
        })
        .unwrap();

    let rancher_role_templates =
        get_role_templates(configuration, None, None, None, None, None, None)
            .await
            .map_err(|e| {
                Box::<dyn Error>::from(format!("Failed to get role templates: {:?}", e))
            })?;

    let rrt: Vec<IoCattleManagementv3RoleTemplate> = rancher_role_templates.items.clone();

    // let cluster_projects: HashMap<String, (IoCattleManagementv3Project, Vec<IoCattleManagementv3ProjectRoleTemplateBinding>)> = HashMap::new();

    let rancher_projects = project::get_projects(
        configuration,
        cluster_id,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .map_err(|e| Box::<dyn Error>::from(format!("Failed to get projects: {:?}", e)))?;

    let mut rancher_cluster_config = RancherClusterConfig {
        cluster: rancher_cluster,
        role_templates: rrt,
        projects: HashMap::new(),
    };

    let rprojects: Vec<IoCattleManagementv3Project> = rancher_projects.items.clone();

    for rproject in rprojects {
        let project = rproject.clone();
        let project_id = project
            .metadata
            .as_ref()
            .and_then(|m| m.name.as_deref())
            .unwrap();
        let rancher_project_role_template_bindings =
            prtb::get_namespaced_project_role_template_bindings(
                configuration,
                project_id,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .map_err(|e| {
                Box::<dyn Error>::from(format!(
                    "Failed to get project role template bindings: {:?}",
                    e
                ))
            })?;
        let rprtbs: Vec<IoCattleManagementv3ProjectRoleTemplateBinding> =
            rancher_project_role_template_bindings.items.clone();

        rancher_cluster_config
            .projects
            .insert(project_id.to_string(), (project, rprtbs));
    }
    return Ok(rancher_cluster_config);
}

// load the entire configuration from the base path
///
/// # Arguments
/// `path`: The path for the directory to load the configuration from
/// `endpoint_url`: The endpoint URL to load the configuration from (relates to the directory name where the whole cluster configuration is stored)
/// `cluster_id`: The cluster ID to load the configuration from
/// `file_format`: The file format to load the configuration from
/// # Returns
/// `Option<ClusterConfig>`: The configuration object
#[async_backtrace::framed]
pub async fn load_configuration(
    path: &Path,
    endpoint_url: &str,
    cluster_id: &str,
    file_format: &FileFormat,
) -> Result<Option<ClusterConfig>, Box<dyn Error>> {
    let endpoint_path = path.join(endpoint_url.replace("https://", "").replace("/", "_"));
    if !endpoint_path.exists() {
        return Err(format!("Configuration path does not exist: {:?}", endpoint_path).into());
    }

    let cluster_folder_path = endpoint_path.join(cluster_id);
    if !cluster_folder_path.exists() {
        return Err(format!("Cluster path does not exist: {:?}", cluster_folder_path).into());
    }

    let cluster_file = cluster_folder_path.join(format!(
        "{}.{}",
        cluster_id,
        file_extension_from_format(file_format)
    ));
    if !cluster_file.exists() {
        return Err(format!("Cluster file does not exist: {:?}", cluster_file).into());
    }

    let cluster_file_content = read_to_string(&cluster_file)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let cluster: Cluster = deserialize_object(&cluster_file_content, file_format)?;

    let mut cluster_config = ClusterConfig {
        cluster: cluster.clone(),
        role_templates: Vec::new(),
        projects: HashMap::new(),
    };

    // Read role templates
    let role_template_path = endpoint_path.join("roles");
    if !role_template_path.exists() {
        return Err(format!(
            "Role template path does not exist: {:?}",
            role_template_path
        )
        .into());
    }

    let mut role_template_dir = read_dir(&role_template_path)
        .await
        .map_err(|e| format!("Failed to read role template directory: {}", e))?;

    let mut role_templates = Vec::new();
    while let Some(entry) = role_template_dir.next_entry().await.map_err(|e| format!("Failed to read role template entry: {}", e))? {
        if entry.file_type().await?.is_file() {
            let content = read_to_string(entry.path())
                .await
                .map_err(|e| format!("Failed to read role template file: {}", e))?;
            let role_template: RoleTemplate = deserialize_object(&content, file_format)?;
            role_templates.push(role_template);
        }
    }
    cluster_config.role_templates = role_templates;

    // Read projects and their PRTBs
    let mut project_dir = read_dir(&cluster_folder_path).await.map_err(|e| format!("Failed to read cluster folder: {}", e))?;

    while let Some(entry) = project_dir.next_entry().await.map_err(|e| format!("Failed to read project entry: {}", e))? {
        if entry.file_type().await?.is_dir() {
            let project_display_name = entry.file_name().to_string_lossy().to_string();
            let project_file = entry.path().join(format!(
                "{}.{}",
                project_display_name,
                file_extension_from_format(file_format)
            ));

            if !project_file.exists() {
                return Err(format!("Project file does not exist: {:?}", project_file).into());
            }

            let content = read_to_string(&project_file)
                .await
                .map_err(|e| format!("Failed to read project file: {}", e))?;
            let project: Project = deserialize_object(&content, file_format)?;

            let project_id = project.id.clone().unwrap_or("default".to_string());

            cluster_config
                .projects
                .insert(project_id.clone(), (project.clone(), Vec::new()));

            let mut prtb_entries = read_dir(entry.path()).await.map_err(|e| format!("Failed to read PRTB directory: {}", e))?;

            let mut prtbs = Vec::new();
            while let Some(prtb_entry) = prtb_entries.next_entry().await.map_err(|e| format!("Failed to read PRTB entry: {}", e))? {
            if prtb_entry.file_type().await?.is_file() && prtb_entry.path() != project_file {
                    let content = read_to_string(prtb_entry.path()).await .map_err(|e| format!("Failed to read PRTB file: {}", e))?;
                    let prtb: ProjectRoleTemplateBinding = deserialize_object(&content, file_format)?;
                prtbs.push(prtb);
                }
            }
            if let Some((_, existing_prtbs)) = cluster_config.projects.get_mut(&project_id) {
                existing_prtbs.extend(prtbs);
            } else {
                return Err(format!("Project not found in cluster config: {}", project_id).into());
            }
        }
    }

    Ok(Some(cluster_config))
}

/// Recursively remove fields from a JSON Value based on a list of dot-separated paths.
/// # Arguments
/// * `value` - The mutable JSON object to clean
/// * `exclude_paths` - A list of dot-separated paths to remove (e.g., ["status", "metadata.creationTimestamp"])
pub fn clean_up_value(value: &mut Value, exclude_paths: &[&str]) {
    for path in exclude_paths {
        let parts: Vec<&str> = path.split('.').collect();
        remove_path_and_return(value, &parts);
    }
}

/// Remove a deeply nested field from a JSON object and remove it.
/// Traverses objects by key. Returns `None` if any key is missing or the path is invalid.
fn remove_path_and_return(value: &mut Value, path: &[&str]) -> Option<Value> {
    if path.is_empty() {
        return None;
    }

    let mut current = value;

    // Traverse to the parent of the key to remove
    for &key in &path[..path.len() - 1] {
        current = current.as_object_mut()?.get_mut(key)?;
    }

    let last_key = *path.last().unwrap();

    // Remove the target key
    current.as_object_mut().unwrap().remove(last_key)
}


// load an object from the file path specified
pub async fn load_object<T: serde::de::DeserializeOwned>(
    file_path: &Path, file_format: &FileFormat
) -> Result<T, Box<dyn std::error::Error>> {
    let contents = read_to_string(file_path).await?;
    match file_format {
        FileFormat::Yaml => Ok(serde_yaml::from_str(&contents)?),
        FileFormat::Json => Ok(serde_json::from_str(&contents)?),
        FileFormat::Toml => Ok(toml::from_str(&contents)?),
    }
    
}

pub async fn create_objects(
    configuration: &Configuration,
    new_files: Vec<(ObjectType, PathBuf)>,
    file_format: &FileFormat,
) -> Vec<Result<CreatedObject, Box<dyn std::error::Error>>> {
    let mut results = Vec::new();

    for (object_type, file_path) in new_files {
        let result = match object_type {
            ObjectType::Cluster => {
                // Not implemented yet
                Err("Cluster creation not implemented".into())
            }

            ObjectType::Project => {
                async {
                    let project = load_object::<Project>(&file_path, file_format).await?;
                    let display_name = project.display_name.clone();
                    let rancher_p = IoCattleManagementv3Project::try_from(project)?;
                    let cluster_name = rancher_p
                        .spec
                        .as_ref()
                        .ok_or("Missing spec")?
                        .cluster_name
                        .clone();
                    let created = create_project(configuration, &cluster_name, rancher_p).await?;
                    println!("Created project: {}", display_name);
                    Ok(CreatedObject::Project(created))
                }
                .await
            }

            ObjectType::RoleTemplate => {
                async {
                    let role_template = load_object::<RoleTemplate>(&file_path, file_format).await?;
                    let display_name = role_template.display_name.clone();
                    let rancher_rt = IoCattleManagementv3RoleTemplate::try_from(role_template)?;
                    let created = create_role_template(configuration, rancher_rt).await?;
                    println!("Created role template: {}", display_name.unwrap_or_default());
                    Ok(CreatedObject::RoleTemplate(created))
                }
                .await
            }

            ObjectType::ProjectRoleTemplateBinding => {
                async {
                    let prtb = load_object::<ProjectRoleTemplateBinding>(&file_path, file_format).await?;
                    let display_name = prtb.id.clone();
                    let rancher_prtb = IoCattleManagementv3ProjectRoleTemplateBinding::try_from(prtb)?;
                    let project_id = rancher_prtb
                        .metadata
                        .as_ref()
                        .and_then(|m| m.namespace.clone())
                        .ok_or("Missing namespace in metadata")?;
                    let created = create_project_role_template_binding(configuration, &project_id, rancher_prtb).await?;
                    println!("Created PRTB: {}", display_name);
                    Ok(CreatedObject::ProjectRoleTemplateBinding(created))
                }
                .await
            }
        };

        results.push(result);
    }

    results
}




/// serialize the object to the file format specified
pub fn serialize_object<T: serde::Serialize>(
    object: &T,
    file_format: &FileFormat,
) -> Result<String, Box<dyn Error>> {
    match file_format {
        FileFormat::Yaml => serde_yaml::to_string(object).map_err(|e| {
            Box::<dyn Error>::from(format!("Failed to serialize object to YAML: {}", e))
        }),
        FileFormat::Json => serde_json::to_string_pretty(object).map_err(|e| {
            Box::<dyn Error>::from(format!("Failed to serialize object to JSON: {}", e))
        }),
        FileFormat::Toml => toml::to_string_pretty(object).map_err(|e| {
            Box::<dyn Error>::from(format!("Failed to serialize object to TOML: {}", e))
        }),
    }
}

// deserialize the project from the format specified
///
/// # Arguments
/// FileFormat: The format of the file to be deserialized
/// object: The object to be deserialized
pub fn deserialize_object<T: serde::de::DeserializeOwned>(
    object: &str,
    file_format: &FileFormat,
) -> Result<T, Box<dyn Error>> {
    match file_format {
        FileFormat::Yaml => serde_yaml::from_str(object).map_err(|e| e.into()),
        FileFormat::Json => serde_json::from_str(object).map_err(|e| e.into()),
        FileFormat::Toml => toml::from_str(object).map_err(|e| e.into()),
    }
}

pub enum ResourceVersionMatch {
    Exact,
    NotOlderThan,
}

impl ResourceVersionMatch {
    fn as_str(&self) -> &'static str {
        match self {
            ResourceVersionMatch::Exact => "Exact",
            ResourceVersionMatch::NotOlderThan => "notOlderThan",
        }
    }
}
impl std::fmt::Display for ResourceVersionMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
impl std::str::FromStr for ResourceVersionMatch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "exact" => Ok(ResourceVersionMatch::Exact),
            "notolderthan" => Ok(ResourceVersionMatch::NotOlderThan),
            _ => Err(()),
        }
    }
}

/// The type of object to be updated in Rancher.
///
/// This enum represents the different types of objects that can be updated in Rancher. It includes:
/// - `Cluster`: Represents a cluster object.
/// - `Project`: Represents a project object.
/// - `RoleTemplate`: Represents a role template object.
/// - `ProjectRoleTemplateBinding`: Represents a project-role-template binding object.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    RoleTemplate,
    Project,
    ProjectRoleTemplateBinding,
    Cluster,
}


pub enum CreatedObject {
    // Cluster(Cluster),
    Project(IoCattleManagementv3Project),
    RoleTemplate(IoCattleManagementv3RoleTemplate),
    ProjectRoleTemplateBinding(IoCattleManagementv3ProjectRoleTemplateBinding),
}

