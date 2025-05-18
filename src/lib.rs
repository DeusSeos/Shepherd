// This file will contain all the functions that will be used to interact and extract from the Rancher API
pub mod cluster;
pub mod config;
pub mod diff;
pub mod file;
pub mod git;
pub mod models;
pub mod project;
pub mod prtb;
pub mod rt;
pub mod update;

use anyhow::{bail, Context, Result};
use file::{file_extension_from_format, FileFormat};
use models::{CreatedObject, ObjectType};
use serde_json::Value;
use tokio::task::JoinHandle;
use std::collections::HashMap;
use std::ops::Deref;
use std::option::Option;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::{create_dir_all, read_dir, read_to_string, write, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;

use cluster::Cluster;
use config::{ClusterConfig, RancherClusterConfig};
use project::{create_project, find_project, Project};
use prtb::{create_project_role_template_binding, ProjectRoleTemplateBinding};
use rt::{create_role_template, find_role_template, get_role_templates, RoleTemplate};

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

        let cluster_file = cluster_path.join(format!(
            "{}.{}",
            cluster.id,
            file_extension_from_format(file_format)
        ));
        write(&cluster_file, serialize_object(cluster, file_format)?)
            .await
            .with_context(|| format!("Failed to write cluster file {:?}", cluster_file))?;

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
        .context("Failed to get projects")?;

        let projects: Vec<Project> = rancher_projects
            .items
            .into_iter()
            .map(|item| item.try_into().context("Failed to convert project"))
            .collect::<Result<_>>()?;

        for project in &projects {
            let project_path = cluster_path.join(&project.display_name);
            if !project_path.exists() {
                create_dir_all(&project_path)
                    .await
                    .context("Failed to create project folder")?;
            }

            let project_file = project_path.join(format!(
                "{}.{}",
                project.display_name,
                file_extension_from_format(file_format)
            ));
            write(&project_file, serialize_object(project, file_format)?)
                .await
                .with_context(|| format!("Failed to write project file {:?}", project_file))?;

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
                let prtb_file = project_path.join(format!(
                    "{}.{}",
                    prtb.id,
                    file_extension_from_format(file_format)
                ));
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
) -> Result<Option<ClusterConfig>> {
    let endpoint_path = path.join(endpoint_url.replace("https://", "").replace("/", "_"));
    if !endpoint_path.exists() {
        bail!("Configuration path does not exist: {:?}", endpoint_path);
    }

    let cluster_folder_path = endpoint_path.join(cluster_id);
    if !cluster_folder_path.exists() {
        bail!("Cluster path does not exist: {:?}", cluster_folder_path);
    }

    let cluster_file = cluster_folder_path.join(format!(
        "{}.{}",
        cluster_id,
        file_extension_from_format(file_format)
    ));
    if !cluster_file.exists() {
        bail!("Cluster file does not exist: {:?}", cluster_file);
    }

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
        bail!(
            "Role template path does not exist: {:?}",
            role_template_path
        );
    }

    let mut role_template_dir = read_dir(&role_template_path).await.with_context(|| {
        format!(
            "Failed to read role template directory: {:?}",
            role_template_path
        )
    })?;

    let mut role_templates = Vec::new();
    while let Some(entry) = role_template_dir
        .next_entry()
        .await
        .with_context(|| "Failed to read role template entry")?
    {
        if entry
            .file_type()
            .await
            .with_context(|| "Failed to get role template file type")?
            .is_file()
        {
            let content = read_to_string(entry.path()).await.with_context(|| {
                format!("Failed to read role template file: {:?}", entry.path())
            })?;
            let role_template: RoleTemplate = deserialize_object(&content, file_format)
                .with_context(|| {
                    format!(
                        "Failed to deserialize role template file: {:?}",
                        entry.path()
                    )
                })?;
            role_templates.push(role_template);
        }
    }
    cluster_config.role_templates = role_templates;

    // Read projects and their PRTBs
    let mut project_dir = read_dir(&cluster_folder_path)
        .await
        .with_context(|| format!("Failed to read cluster folder: {:?}", cluster_folder_path))?;

    while let Some(entry) = project_dir
        .next_entry()
        .await
        .with_context(|| "Failed to read project entry")?
    {
        if entry
            .file_type()
            .await
            .with_context(|| "Failed to get project file type")?
            .is_dir()
        {
            let project_display_name = entry.file_name().to_string_lossy().to_string();
            let project_file = entry.path().join(format!(
                "{}.{}",
                project_display_name,
                file_extension_from_format(file_format)
            ));

            if !project_file.exists() {
                bail!("Project file does not exist: {:?}", project_file);
            }

            let content = read_to_string(&project_file)
                .await
                .with_context(|| format!("Failed to read project file: {:?}", project_file))?;
            let project: Project =
                deserialize_object(&content, file_format).with_context(|| {
                    format!("Failed to deserialize project file: {:?}", project_file)
                })?;

            let project_id = project.id.clone().unwrap_or_else(|| "default".to_string());

            cluster_config
                .projects
                .insert(project_id.clone(), (project.clone(), Vec::new()));

            let mut prtb_entries = read_dir(entry.path())
                .await
                .with_context(|| format!("Failed to read PRTB directory: {:?}", entry.path()))?;

            let mut prtbs = Vec::new();
            while let Some(prtb_entry) = prtb_entries
                .next_entry()
                .await
                .with_context(|| "Failed to read PRTB entry")?
            {
                if prtb_entry
                    .file_type()
                    .await
                    .with_context(|| "Failed to get PRTB file type")?
                    .is_file()
                    && prtb_entry.path() != project_file
                {
                    let content = read_to_string(prtb_entry.path()).await.with_context(|| {
                        format!("Failed to read PRTB file: {:?}", prtb_entry.path())
                    })?;
                    let prtb: ProjectRoleTemplateBinding =
                        deserialize_object(&content, file_format).with_context(|| {
                            format!("Failed to deserialize PRTB file: {:?}", prtb_entry.path())
                        })?;
                    prtbs.push(prtb);
                }
            }

            if let Some((_, existing_prtbs)) = cluster_config.projects.get_mut(&project_id) {
                existing_prtbs.extend(prtbs);
            } else {
                bail!("Project not found in cluster config: {}", project_id);
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
) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
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
/// * `fetch_fn` - An async closure that attempts to fetch the object and returns `Ok(Some(obj))` if found, `Ok(None)` if not yet available, or `Err(e)` on failure
///
/// # Returns
/// * `Ok(T)` - If the object was eventually found
/// * `Err(Box<dyn std::error::Error + Send + Sync>)` - If polling fails or times out
///
pub async fn wait_for_object_ready<T, F, Fut>(
    max_retries: usize,
    delay: Duration,
    mut fetch_fn: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Option<T>, Box<dyn std::error::Error + Send + Sync>>>,
    T: Send + Sync + 'static,
{
    for attempt in 0..max_retries {
        match fetch_fn().await {
            Ok(Some(obj)) => return Ok(obj),
            Ok(None) => {
                if attempt + 1 == max_retries {
                    break;
                }
                sleep(delay).await;
            }
            Err(e) => {
                if attempt + 1 == max_retries {
                    return Err(e);
                }
                sleep(delay).await;
            }
        }
    }
    Err("Timed out waiting for object to become ready".into())
}

pub async fn create_objects(
    configuration: Arc<Configuration>,
    new_files: Vec<(ObjectType, PathBuf)>,
    file_format: FileFormat,
) -> Vec<Result<(PathBuf, CreatedObject), Box<dyn std::error::Error + Send + Sync>>> {
    // Enforce creation order of RoleTemplates, then Projects, then ProjectRoleTemplateBindings
    let mut new_files = new_files;
    new_files.sort_by(|a, b| {
        let a_priority = match a.0 {
            ObjectType::RoleTemplate => 0,
            ObjectType::Project => 1,
            ObjectType::ProjectRoleTemplateBinding => 2,
            ObjectType::Cluster => 3,
        };
        let b_priority = match b.0 {
            ObjectType::RoleTemplate => 0,
            ObjectType::Project => 1,
            ObjectType::ProjectRoleTemplateBinding => 2,
            ObjectType::Cluster => 3,
        };
        a_priority.cmp(&b_priority)
    });

    let mut handles_role_templates = Vec::with_capacity(
        new_files
            .iter()
            .filter(|(object_type, _)| *object_type == ObjectType::RoleTemplate)
            .count(),
    );
    let mut handles_projects = Vec::with_capacity(
        new_files
            .iter()
            .filter(|(object_type, _)| *object_type == ObjectType::Project)
            .count(),
    );
    let mut handles_prtbs = Vec::with_capacity(
        new_files
            .iter()
            .filter(|(object_type, _)| *object_type == ObjectType::ProjectRoleTemplateBinding)
            .count(),
    );

    for (object_type, file_path) in new_files {
        let config = configuration.clone();
        let format = file_format;
        match object_type {
            ObjectType::RoleTemplate => {
                handles_role_templates.push(tokio::spawn(async move {
                    let result: Result<
                        (PathBuf, CreatedObject),
                        Box<dyn std::error::Error + Send + Sync>,
                    > = match object_type {
                        ObjectType::RoleTemplate => {
                            let role_template =
                                load_object::<RoleTemplate>(&file_path, &format).await?;
                            let display_name = role_template.display_name.clone();
                            let rancher_rt =
                                IoCattleManagementv3RoleTemplate::try_from(role_template)?;
                            let created = create_role_template(&*config, rancher_rt).await?;

                            let rt_name = created
                                .metadata
                                .as_ref()
                                .and_then(|m| m.name.as_deref())
                                .ok_or("Missing metadata.name in created role template")?;

                            let resource_version = created
                                .metadata
                                .as_ref()
                                .and_then(|m| m.resource_version.as_deref());

                            // poll for the role template to become available
                            let created =
                                wait_for_object_ready(10, Duration::from_secs(1), || {
                                    let config = config.clone();
                                    let rt_name = rt_name.to_string();
                                    let resource_version = resource_version.map(|s| s.to_string());

                                    async move {
                                        match find_role_template(
                                            &*config,
                                            &rt_name,
                                            resource_version.as_deref(),
                                        )
                                        .await
                                        {
                                            Ok(rt) => Ok(Some(rt)),
                                            Err(e) => Err(Box::new(e)
                                                as Box<dyn std::error::Error + Send + Sync>),
                                        }
                                    }
                                })
                                .await?;

                            println!(
                                "Created role template: {}",
                                display_name.unwrap_or_default()
                            );
                            Ok((file_path, CreatedObject::RoleTemplate(created)))
                        }
                        _ => unreachable!(),
                    };
                    result
                }));
            }
            ObjectType::Project => {
                handles_projects.push(tokio::spawn(async move {
                    let result: Result<
                        (PathBuf, CreatedObject),
                        Box<dyn std::error::Error + Send + Sync>,
                    > = match object_type {
                        ObjectType::Project => {
                            let project = load_object::<Project>(&file_path, &format).await?;
                            let display_name = project.display_name.clone();
                            let rancher_p = IoCattleManagementv3Project::try_from(project)?;
                            let cluster_name = rancher_p
                                .spec
                                .as_ref()
                                .ok_or("Missing spec")?
                                .cluster_name
                                .clone();
                            let created = create_project(&*config, &cluster_name, rancher_p).await?;
                            println!("Created project: {}", display_name);
                            Ok((file_path, CreatedObject::Project(created)))
                        }
                        _ => unreachable!(),
                    };
                    result
                }));
            }
            ObjectType::ProjectRoleTemplateBinding => {
                handles_prtbs.push(tokio::spawn(async move {
                    let result: Result<
                        (PathBuf, CreatedObject),
                        Box<dyn std::error::Error + Send + Sync>,
                    > = match object_type {
                        ObjectType::ProjectRoleTemplateBinding => {
                            let prtb =
                                load_object::<ProjectRoleTemplateBinding>(&file_path, &format)
                                    .await?;
                            let display_name = prtb.id.clone();
                            let rancher_prtb =
                                IoCattleManagementv3ProjectRoleTemplateBinding::try_from(prtb)?;
                            let project_id = rancher_prtb
                                .metadata
                                .as_ref()
                                .and_then(|m| m.namespace.clone())
                                .ok_or("Missing namespace in metadata")?;
                            let created = create_project_role_template_binding(
                                &*config,
                                &project_id,
                                rancher_prtb,
                            )
                            .await?;
                            println!("Created PRTB: {}", display_name);
                            Ok((
                                file_path,
                                CreatedObject::ProjectRoleTemplateBinding(created),
                            ))
                        }
                        _ => unreachable!(),
                    };
                    result
                }));
            }
            _ => unreachable!(),
        }
    }

    // Wait for all role templates to be created
    let mut results = Vec::with_capacity(handles_role_templates.len());
    for handle in handles_role_templates {
        match handle.await {
            Ok(res) => results.push(res),
            Err(join_err) => results.push(Err(Box::new(join_err))),
        }
    }

    // Wait for all projects to be created
    for handle in handles_projects {
        match handle.await {
            Ok(res) => 
                // Object was create successfully, poll for it to be ready
                match res {

                    Ok((_, CreatedObject::Project(created))) => {
                        let p_name = created
                            .metadata
                            .as_ref()
                            .and_then(|m| m.name.as_deref())
                            .ok_or("Missing metadata.name in created project").unwrap();

                        let resource_version = created
                            .metadata
                            .as_ref()
                            .and_then(|m| m.resource_version.as_deref());

                        let c_name = created
                            .spec
                            .as_ref()
                            .ok_or("Missing spec").unwrap()
                            .cluster_name
                            .clone();

                        // poll for the project to become available
                        let created =
                            wait_for_object_ready(10, Duration::from_secs(1), || {
                                let config = configuration.clone();
                                let p_name = p_name.to_string();
                                let c_name = c_name.to_string();
                                let resource_version = resource_version.map(|s| s.to_string());

                                async move {
                                    match find_project(
                                        &*config,
                                        &c_name,
                                        &p_name,
                                        resource_version.as_deref(),
                                    )
                                    .await
                                    {
                                        Ok(p) => Ok(Some(p)),
                                        Err(e) => Err(Box::new(e)
                                            as Box<dyn std::error::Error + Send + Sync>),
                                    }
                                }
                            })
                            .await;

                            // if the project was created successfully, log that it was created
                            match created {
                                Ok(_) => {
                                    println!("Polled and verified project: {}", p_name);
                                },
                                Err(_) => {
                                    println!("Error creating project: {}", p_name);
                                }
                            }

                },
                Ok(res) => results.push(Ok(res)),

                Err(e) => results.push(Err(e))
            },
            Err(join_err) => results.push(Err(Box::new(join_err))),
        }
    }

    // Wait for all project role template bindings to be created
    for handle in handles_prtbs {
        match handle.await {
            Ok(res) => results.push(res),
            Err(join_err) => results.push(Err(Box::new(join_err))),
        }
    }
    results
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
) -> Result<T> {
    match file_format {
        FileFormat::Yaml => serde_yaml::from_str(object).map_err(|e| e.into()),
        FileFormat::Json => serde_json::from_str(object).map_err(|e| e.into()),
        FileFormat::Toml => toml::from_str(object).map_err(|e| e.into()),
    }
}
