// This file will contain all the functions that will be used to interact and extract from the Rancher API
pub mod utils{
    pub mod diff;
    pub mod file;
    pub mod git;
    pub mod logging;
}

pub mod resources {
    pub mod project;
    pub mod cluster;
    pub mod prtb;
    pub mod rt;
}

pub mod api {
    pub mod config;
    pub mod errors;
    pub mod client_info;
    pub mod client;
}


pub mod models;
pub mod modify;
pub mod traits;

use anyhow::{bail, Context, Result};

use utils::file::{file_extension_from_format, get_file_name_for_object, FileFormat};
use utils::logging::log_api_error;

use models::{ConversionError, CreatedObject, ObjectType};


use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::option::Option;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::{create_dir_all, read_dir, read_to_string, write};
use tokio::time::sleep;
use tracing::{debug, trace, error, info, warn};

use api::config::{ClusterConfig, RancherClusterConfig};
use resources::cluster::{self, Cluster, get_clusters};
use resources::project::{find_project, get_projects, Project};
use resources::prtb::{get_namespaced_project_role_template_bindings, ProjectRoleTemplateBinding};
use resources::rt::{find_role_template, get_role_templates, RoleTemplate};

use rancher_client::apis::configuration::Configuration;
use rancher_client::models::{
    IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding,
    IoCattleManagementv3RoleTemplate,
};

include!(concat!(env!("OUT_DIR"), "/client_info.rs"));



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
) -> Result<()> {
    let rancher_cluster = cluster::get_clusters(configuration)
        .await
        .context("Failed to get clusters")?;

    let rancher_role_templates =
        get_role_templates(configuration, None, None, None, None, None, None)
            .await
            .context("Failed to get role templates")?;

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
            .context("Failed to create base folder")?;
    }

    let role_template_path = base_path.join("roles");
    if !role_template_path.exists() {
        create_dir_all(&role_template_path)
            .await
            .context("Failed to create role templates folder")?;
    }

    let role_templates: Vec<RoleTemplate> = rancher_role_templates
        .items
        .into_iter()
        .map(|item| item.try_into().context("Failed to convert role template"))
        .collect::<Result<_>>()?;

    for role_template in &role_templates {
        let role_template_file = role_template_path.join(get_file_name_for_object(&role_template.id, &ObjectType::RoleTemplate, file_format));
        write(
            &role_template_file,
            serialize_object(role_template, file_format)?,
        )
        .await
        .with_context(|| format!("Failed to write file {:?}", role_template_file))?;
    }

    let clusters: Vec<Cluster> = rancher_cluster
        .items
        .into_iter()
        .map(|item| item.try_into().context("Failed to convert cluster"))
        .collect::<Result<_>>()?;

    for cluster in &clusters {
        let cluster_path = base_path.join(&cluster.id);
        if !cluster_path.exists() {
            create_dir_all(&cluster_path)
                .await
                .context("Failed to create cluster folder")?;
        }

        let cluster_file = cluster_path.join(get_file_name_for_object(&cluster.id, &ObjectType::Cluster, file_format));
        write(&cluster_file, serialize_object(cluster, file_format)?)
            .await
            .with_context(|| format!("Failed to write cluster file {:?}", cluster_file))?;

        let rancher_projects = get_projects(
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
        .context("Failed to get projects")?;

        let projects: Vec<Project> = rancher_projects
            .items
            .into_iter()
            .map(|item| item.try_into().context("Failed to convert project"))
            .collect::<Result<_>>()?;

        for project in &projects {
            let project_path = cluster_path.join(&project.id.clone().unwrap());
            if !project_path.exists() {
                create_dir_all(&project_path)
                    .await
                    .context("Failed to create project folder")?;
            }

            let project_file = project_path.join(get_file_name_for_object(&project.id.clone().unwrap(), &ObjectType::Project, file_format));
            write(&project_file, serialize_object(project, file_format)?)
                .await
                .with_context(|| format!("Failed to write project file {:?}", project_file))?;

            let rancher_prtbs = get_namespaced_project_role_template_bindings(
                configuration,
                &project.id.clone().unwrap(),
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .context("Failed to get project role template bindings")?;

            let prtbs: Vec<ProjectRoleTemplateBinding> = rancher_prtbs
                .items
                .into_iter()
                .map(|item| {
                    item.try_into()
                        .context("Failed to convert project role template binding")
                })
                .collect::<Result<_>>()?;

            for prtb in &prtbs {
                let prtb_file = project_path.join(get_file_name_for_object(&prtb.id, &ObjectType::ProjectRoleTemplateBinding, file_format));
                write(&prtb_file, serialize_object(prtb, file_format)?)
                    .await
                    .with_context(|| format!("Failed to write PRTB file {:?}", prtb_file))?;
            }
        }
    }

    Ok(())
}

#[async_backtrace::framed]
pub async fn load_configuration_from_rancher(
    configuration: &Configuration,
    cluster_id: &str,
) -> Result<RancherClusterConfig> {
    // Get the current configuration from the Rancher API
    let rancher_clusters = cluster::get_clusters(configuration)
        .await
        .context("Failed to get clusters")?;

    let rancher_cluster = rancher_clusters
        .items
        .into_iter()
        .find(|cluster| {
            cluster.metadata.as_ref().and_then(|m| m.name.as_deref()) == Some(cluster_id)
        })
        .ok_or_else(|| anyhow::anyhow!("Cluster with id '{}' not found", cluster_id))?;

    let rancher_role_templates =
        get_role_templates(configuration, None, None, None, None, None, None)
            .await
            .context("Failed to get role templates")?;

    let rrt: Vec<IoCattleManagementv3RoleTemplate> = rancher_role_templates.items.clone();

    let rancher_projects = get_projects(
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
    .context("Failed to get projects")?;

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
            .ok_or_else(|| anyhow::anyhow!("Project missing metadata name"))?;

        let rancher_project_role_template_bindings =
            get_namespaced_project_role_template_bindings(
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
            .context(format!(
                "Failed to get project role template bindings for project '{}'",
                project_id
            ))?;

        let rprtbs: Vec<IoCattleManagementv3ProjectRoleTemplateBinding> =
            rancher_project_role_template_bindings.items.clone();

        rancher_cluster_config
            .projects
            .insert(project_id.to_string(), (project, rprtbs));
    }

    Ok(rancher_cluster_config)
}

pub async fn load_configuration(
    path: &Path,
    endpoint_url: &str,
    cluster_id: &str,
    file_format: &FileFormat,
) -> Result<Option<ClusterConfig>> {
    let endpoint_path = path.join(endpoint_url.replace("https://", "").replace("/", "_"));
    if !endpoint_path.exists() {
        bail!("Configuration path does not exist: {:?}", endpoint_path);
    }

    let cluster_folder_path = endpoint_path.join(cluster_id);
    if !cluster_folder_path.exists() {
        bail!("Cluster path does not exist: {:?}", cluster_folder_path);
    }

    let extension = file_extension_from_format(file_format);
    let cluster_file = cluster_folder_path.join(format!("{}.cluster.{}", cluster_id, extension));
    if !cluster_file.exists() {
        bail!("Cluster file does not exist: {:?}", cluster_file);
    }

    // info!("Loading cluster configuration from file: {:?}", cluster_file);
    info!(path = %cluster_file.display(), "Reading cluster file");
    let cluster_file_content = read_to_string(&cluster_file)
        .await
        .with_context(|| format!("Failed to read cluster file: {:?}", cluster_file))?;
    let cluster: Cluster = deserialize_object(&cluster_file_content, file_format)
        .with_context(|| format!("Failed to deserialize cluster file: {:?}", cluster_file))?;

    let mut cluster_config = ClusterConfig {
        cluster: cluster.clone(),
        role_templates: Vec::new(),
        projects: std::collections::HashMap::new(),
    };

    // Read role templates
    let role_template_path = endpoint_path.join("roles");
    if !role_template_path.exists() {
        bail!("Role template path does not exist: {:?}", role_template_path);
    }

    let mut role_templates = Vec::new();
    let mut rd = read_dir(&role_template_path).await?;
    while let Some(entry) = rd.next_entry().await? {
        if entry.file_type().await?.is_file() {
            let rt_file_name = entry.file_name();
            let file_name = rt_file_name.to_string_lossy();
            if file_name.ends_with(&format!(".rt.{}", extension)) {
                let content = read_to_string(entry.path()).await?;
                let role_template: RoleTemplate = deserialize_object(&content, file_format)?;
                role_templates.push(role_template);
            }
        }
    }
    cluster_config.role_templates = role_templates;

    // Read projects
    let mut rd = read_dir(&cluster_folder_path).await?;
    while let Some(entry) = rd.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            let project_folder_path = entry.path();
            let project_id = entry.file_name().to_string_lossy().to_string();

            // Look for project file with new naming convention
            let project_file = project_folder_path.join(format!("{}.project.{}", project_id, extension));
            if project_file.exists() {
                info!("Loading project configuration from file: {:?}", project_file);
                let content = read_to_string(&project_file).await
                    .with_context(|| format!("Failed to read project file: {:?}", project_file))?;
                let project: Project = deserialize_object(&content, file_format)
                    .with_context(|| format!("Failed to deserialize project file: {:?}", project_file))?;

                // Read PRTBs
                let mut prtbs = Vec::new();
                let mut prd = read_dir(&project_folder_path).await?;
                while let Some(prtb_entry) = prd.next_entry().await? {
                    if prtb_entry.file_type().await?.is_file() {
                        let prtb_file_name = prtb_entry.file_name();
                        let file_name = prtb_file_name.to_string_lossy();
                        if file_name.ends_with(&format!(".prtb.{}", extension)) {
                            let content = read_to_string(prtb_entry.path()).await
                                .with_context(|| format!("Failed to read PRTB file: {:?}", prtb_entry.path()))?;
                            let prtb: ProjectRoleTemplateBinding = deserialize_object(&content, file_format)
                                .with_context(|| format!("Failed to deserialize PRTB file: {:?}", prtb_entry.path()))?;
                            prtbs.push(prtb);
                        }
                    }
                }

                cluster_config.projects.insert(project_id, (project, prtbs));
            } else {
                warn!("Project file not found: {:?}", project_file);
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
    file_path: &Path,
    file_format: &FileFormat,
) -> Result<T> {
    let contents = read_to_string(file_path).await?;
    match file_format {
        FileFormat::Yaml => Ok(serde_yaml::from_str(&contents)?),
        FileFormat::Json => Ok(serde_json::from_str(&contents)?),
        FileFormat::Toml => Ok(toml::from_str(&contents)?),
    }
}

/// Polls until a Rancher object becomes available or a timeout occurs.
///
/// # Arguments
/// * `max_retries` - Number of times to retry
/// * `delay` - Duration between retries
/// * `fetch_fn` - An async closure that attempts to fetch the object and returns `Ok(T)` if found or `Err(anyhow::Error)` on failure
/// * `operation_name` - Name of the operation for logging purposes
///
/// # Returns
/// * `Ok(T)` - If the object was eventually found
/// * `Err(anyhow::Error)` - If polling fails or times out
///
pub async fn wait_for_object_ready<T, F, Fut>(
    max_retries: usize,
    delay: Duration,
    mut fetch_fn: F,
    operation_name: &str,
) -> Result<T, anyhow::Error>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, anyhow::Error>>
{

    for attempt in 0..max_retries {
        trace!("Attempt {}/{} for {}", attempt + 1, max_retries, operation_name);
        
        match fetch_fn().await {
            Ok(obj) => {
                debug!("Successfully retrieved object on attempt {}/{}", attempt + 1, max_retries);
                return Ok(obj);
            }
            Err(e) => {
                // Check if this is a "not found" error that we should retry
                let is_not_found = e.to_string().contains("not found");
                
                if attempt + 1 == max_retries {
                    let err = anyhow::anyhow!("Timed out waiting for object: {}", e);
                    log_api_error(&format!("wait_for_object_ready:{}", operation_name), &err);
                    return Err(err);
                }
                
                if is_not_found {
                    trace!("Object not found on attempt {}/{}, waiting to retry...", attempt + 1, max_retries);
                } else {
                    debug!("Error on attempt {}/{}: {}", attempt + 1, max_retries, e);
                }
                
                tokio::time::sleep(delay).await;
            }
        }
    }
    
    let err = anyhow::anyhow!("Timed out waiting for object to become ready after {} attempts", max_retries);
    log_api_error(&format!("wait_for_object_ready:{}", operation_name), &err);
    Err(err)
}



/// Await all of the given handles, collecting their results into a vector.
///
/// The output vector will contain the same number of elements as the input vector,
/// and the elements will be in the same order. If any of the handles
/// error, the error will be propagated into the output vector.
///
/// # Example
///
///
async fn await_handles(
    handles: Vec<tokio::task::JoinHandle<anyhow::Result<(PathBuf, CreatedObject)>,>>,
) -> Vec<anyhow::Result<(PathBuf, CreatedObject)>> {
    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        match handle.await {
            Ok(res) => results.push(res),
            Err(join_err) => results.push(Err(anyhow::anyhow!(join_err))),
        }
    }
    results
}

/// Poll a role template until it is ready. This function is used to block until
/// a role template is created successfully.
///
/// # Arguments
///
/// * `config`: The config object to use for connecting to the rancher server.
///
/// * `created`: The created role template that we want to poll. The `metadata` field of
///   `created` must contain a valid `name` field.
///
/// # Errors
///
/// If the polling fails for any reason, or if the object is not created successfully,
/// an error will be returned.
///
async fn poll_role_template_ready(
    config: Arc<Configuration>,
    created: &IoCattleManagementv3RoleTemplate,
) -> Result<IoCattleManagementv3RoleTemplate, anyhow::Error> {
    let rt_name = created
        .metadata
        .as_ref()
        .and_then(|m| m.name.as_deref())
        .ok_or_else(|| anyhow::anyhow!("Missing metadata.name in created role template"))?;

    let resource_version = created
        .metadata
        .as_ref()
        .and_then(|m| m.resource_version.as_deref());

    wait_for_object_ready(
        10, 
        Duration::from_secs(1), 
        || {
            let rt_name = rt_name.to_string();
            let resource_version = resource_version.map(|s| s.to_string());
            let config = config.clone();

            async move {
                find_role_template(&config, &rt_name, resource_version.as_deref()).await
            }
        },
        "role_template"
    )
    .await
}

/// Poll a project until it is ready. This function is used to block until
/// a project is created successfully.
///
/// # Arguments
///
/// * `config`: The configuration to use for the request
/// * `created`: The created project that we want to poll.
///
/// # Returns
///
/// * `IoCattleManagementv3Project` - The project once it's ready
///
/// # Errors
///
/// * `anyhow::Error` - If the polling fails or times out
///
#[async_backtrace::framed]
async fn poll_project_ready(
    config: Arc<Configuration>,
    created: &IoCattleManagementv3Project,
) -> Result<IoCattleManagementv3Project, anyhow::Error> {
    let p_name = created
        .metadata
        .as_ref()
        .and_then(|m| m.name.as_deref())
        .ok_or_else(|| anyhow::anyhow!("Missing metadata.name in created project"))?;

    let resource_version = created
        .metadata
        .as_ref()
        .and_then(|m| m.resource_version.as_deref());

    let c_name = created
        .spec
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing spec in created project"))?
        .cluster_name
        .clone();

    wait_for_object_ready(
        10, 
        Duration::from_secs(1), 
        || {
            let p_name = p_name.to_string();
            let c_name = c_name.to_string();
            let resource_version = resource_version.map(|s| s.to_string());
            let config = config.clone();

            async move {
                find_project(&config, &c_name, &p_name, resource_version.as_deref()).await
            }
        },
        "project"
    )
    .await
}

/// Retries an async operation up to `max_retries` times with a delay between attempts.
///
/// # Arguments
/// * `label` - A string label for logging (e.g. "create_prtb")
/// * `max_retries` - Maximum number of attempts
/// * `delay` - Delay between retries
/// * `op` - Async closure that returns a `Result`
/// * `should_retry` - Function that inspects the error and decides whether to retry
///
/// # Returns
/// * `Ok(T)` if the operation eventually succeeds
/// * `Err(E)` if all attempts fail or retry condition is not met
pub async fn retry_async<T, E, F, Fut, R>(
    label: &str,
    max_retries: usize,
    delay: Duration,
    mut op: F,
    should_retry: R,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    R: Fn(&E) -> bool,
    E: std::fmt::Display,
{
    for attempt in 1..=max_retries {
        match op().await {
            Ok(result) => {
                if attempt > 1 {
                    info!(
                        operation = %label,
                        attempt,
                        total_attempts = max_retries,
                        "Retry succeeded"
                    );
                }
                debug!(operation = %label, "Operation succeeded");
                return Ok(result);
            }
            Err(e) => {
                let retry = should_retry(&e);
                if retry && attempt < max_retries {
                    warn!(
                        operation = %label,
                        attempt,
                        total_attempts = max_retries,
                        error = %e,
                        "Retrying after {:?}",
                        delay
                    );
                    sleep(delay).await;
                } else {
                    error!(
                        operation = %label,
                        attempt,
                        total_attempts = max_retries,
                        error = %e,
                        "Giving up"
                    );
                    return Err(e);
                }
            }
        }
    }

    unreachable!("retry_async: loop should return on final attempt")
}


/// serialize the object to the file format specified
pub fn serialize_object<T: serde::Serialize>(
    object: &T,
    file_format: &FileFormat,
) -> Result<String> {
    match file_format {
        FileFormat::Yaml => {
            serde_yaml::to_string(object).context("Failed to serialize object to YAML")
        }
        FileFormat::Json => {
            serde_json::to_string_pretty(object).context("Failed to serialize object to JSON")
        }
        FileFormat::Toml => {
            toml::to_string_pretty(object).context("Failed to serialize object to TOML")
        }
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
) -> Result<T, ConversionError> {
    match file_format {
        FileFormat::Yaml => serde_yaml::from_str(object).map_err(|e| e.into()),
        FileFormat::Json => serde_json::from_str(object).map_err(|e| e.into()),
        FileFormat::Toml => toml::from_str(object).map_err(|e| e.into()),
    }
}
