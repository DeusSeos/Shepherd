use crate::{poll_project_ready, poll_role_template_ready, retry_async, RoleTemplate};
use crate::diff::compute_cluster_diff;
use crate::models::{ConversionError, CreatedObject, MinimalObject};
use crate::project::{create_project, delete_project, update_project};
use crate::prtb::{delete_project_role_template_binding, update_project_role_template_binding};
use crate::rt::{delete_role_template, update_role_template};
use crate::{await_handles, load_configuration, load_configuration_from_rancher, load_object, ObjectType};
use crate::config::RancherClusterConfig;
use crate::file::FileFormat;

use rancher_client::apis::configuration::Configuration;
use rancher_client::apis::Error;
use rancher_client::models::{IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding, IoCattleManagementv3RoleTemplate, IoK8sApimachineryPkgApisMetaV1ObjectMeta};
use reqwest::StatusCode;

use futures::{stream, FutureExt, StreamExt};
use tracing::{debug, error, info};


use crate::project::Project;
use crate::prtb::{create_project_role_template_binding, ProjectRoleTemplateBinding};
use crate::rt::create_role_template;


use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

    /// Compares the stored configuration with the live Rancher configuration and updates the differences.
    ///
    /// # Arguments
    /// * `configuration`: The configuration object to use for connecting to Rancher
    /// * `config_folder_path`: The path to the folder containing the stored configuration
    /// * `cluster_id`: The ID of the cluster to load the stored configuration from
    /// * `file_format`: The file format to load the stored configuration from
    ///
    /// # Returns
    /// `Vec<Result<CreatedObject, Box<dyn std::error::Error + Send + Sync>>>`: A vector of results containing the created objects
pub async fn compare_and_update_configurations(
    configuration: Arc<Configuration>,
    config_folder_path: &Path,
    cluster_id: &str,
    file_format: &FileFormat,
) -> Vec<Result<CreatedObject, Box<dyn std::error::Error + Send + Sync>>> {
    // Load the stored configuration
    let stored_config = load_configuration(config_folder_path, &configuration.base_path, cluster_id, file_format).await.unwrap().unwrap();
    debug!("Loaded stored configuration for cluster `{}`: {} ", cluster_id, stored_config);
    let stored_config: RancherClusterConfig = RancherClusterConfig::try_from(stored_config).unwrap();


    // Load the live Rancher configuration
    let live_config = load_configuration_from_rancher(&configuration, cluster_id).await.unwrap();

    // Compute the differences
    let diffs = compute_cluster_diff(&serde_json::to_value(&live_config).unwrap(), &serde_json::to_value(&stored_config).unwrap());
    info!("Generated diffs for cluster `{}`: {:#?} ", cluster_id, diffs);

    let mut results: Vec<Result<CreatedObject, Box<dyn std::error::Error + Send + Sync>>> = Vec::new();

    // Iterate through the differences and handle them use tokio to do them in parallel
    let mut handles = Vec::with_capacity(diffs.len());
    for ((object_type, object_id, namespace), diff_value) in diffs {
        let handle = tokio::spawn(handle_diff(configuration.clone(), object_type, object_id, namespace, diff_value));
        handles.push(handle);
    }
    for result in stream::iter(handles).buffer_unordered(8).collect::<Vec<_>>().await {
        match result {
            Ok(object) => results.push(object),
            Err(e) => results.push(Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>))
        }
    }

    results
}

async fn handle_diff(
    configuration: Arc<Configuration>,
    object_type: ObjectType,
    object_id: String,
    namespace: Option<String>,
    diff_value: Value,
) -> Result<CreatedObject, Box<dyn std::error::Error + Send + Sync>> {
    match object_type {
        ObjectType::Project => {
            let ns = namespace.as_deref().unwrap_or("<no-namespace>");
            debug!("Updating project `{}` in cluster `{} with diff: {:#?}`", object_id, ns, diff_value);
            let object = update_project(&configuration, ns, &object_id, diff_value).await;
            match object {
                Ok(object) => {
                    info!("Updated project `{}` ({}) in cluster `{}`", object_id, object.spec.as_ref().unwrap().display_name, ns);
                    Ok(CreatedObject::Project(object))
                },
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
        }

        ObjectType::RoleTemplate => {
            info!("Update role-template `{}`", object_id);
            debug!("Update role-template `{}` with diff: {:#?} ", object_id, diff_value);
            let object = update_role_template(&configuration, &object_id, diff_value).await;
            match object {
                Ok(object) => Ok(CreatedObject::RoleTemplate(object)),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
        }

        ObjectType::ProjectRoleTemplateBinding => {
            let ns = namespace.as_deref().unwrap_or("<no-namespace>");
            info!("Updated prtb `{}` in namespace `{}`", object_id, ns);
            debug!("Updated prtb `{}` in namespace `{}` with diff: {:#?} ", object_id, ns, diff_value);
            let object = update_project_role_template_binding(&configuration, ns, &object_id, diff_value).await;
            match object {
                Ok(object) => Ok(CreatedObject::ProjectRoleTemplateBinding(object)),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
        }

        _ => panic!("Unsupported object type: {:?}", object_type),
    }
}



/// Deletes objects from the cluster
/// # Arguments
/// * `configuration` - The configuration object
/// * `deleted_files` - A vector of tuples containing the object type and the minimal object
/// # Returns
/// * `Vec<Result<CreatedObject, Box<dyn std::error::Error + Send + Sync>>>`
pub async fn delete_objects(
    configuration: Arc<Configuration>,
    deleted_files: Vec<(ObjectType, MinimalObject)>,
) -> Vec<Result<CreatedObject, Box<dyn std::error::Error + Send + Sync>>> {
    let mut results = Vec::with_capacity(deleted_files.len());
    for (object_type, minimal_object) in deleted_files {
        match delete_object(&configuration, &object_type, &minimal_object).await {
            Ok(object) => {
                info!("Deleted object: {:#?}", minimal_object);
                results.push(Ok(object))
            },
            Err(e) => { 
                error!("Error deleting {:?} object: {}", minimal_object,  e);
                results.push(Err(e))},
        }
    }
    results
}

/// Deletes an object from the cluster
/// # Arguments
/// * `configuration` - The configuration object
/// * `object_type` - The type of object to delete
/// * `minimal_object` - The minimal object
/// # Returns
/// * `Result<CreatedObject, Box<dyn std::error::Error + Send + Sync>>`
/// 
async fn delete_object(
    configuration: &Arc<Configuration>,
    object_type: &ObjectType,
    minimal_object: &MinimalObject,
) -> Result<CreatedObject, Box<dyn std::error::Error + Send + Sync>> {
    match object_type {
        ObjectType::Project => {
            let cluster_id = minimal_object.namespace.as_deref().unwrap_or("<no-namespace>");
            if minimal_object.object_id.is_none() {
                return Err(Box::new(ConversionError::InvalidValue {
                    field: "object_id".into(),
                    reason: "object_id is required".into(),
                }) as Box<dyn std::error::Error + Send + Sync>);
            }
            info!("Deleting project `{}` from cluster `{}`", minimal_object.object_id.as_ref().unwrap(), cluster_id);
            let object = delete_project(configuration, cluster_id, minimal_object.object_id.as_ref().unwrap().as_ref()).await;
            match object {
                Ok(object) => { 
                    info!("Deleted project `{}` ({}) from cluster `{}`", minimal_object.object_id.as_ref().unwrap(), object.spec.as_ref().unwrap().display_name, cluster_id); 
                    Ok(CreatedObject::Project(object))
            }
                ,
                Err(e) => {
                    error!("Failed to delete project `{}` from cluster `{}`: {}", minimal_object.object_id.as_ref().unwrap(), cluster_id, e);
                    Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)}
            }
        }

        ObjectType::RoleTemplate => {
            if minimal_object.object_id.is_none() {
                return Err(Box::new(ConversionError::InvalidValue {
                    field: "object_id".into(),
                    reason: "object_id is required".into(),
                }) as Box<dyn std::error::Error + Send + Sync>);
            }
            info!("Deleting role-template `{}`", minimal_object.object_id.as_ref().unwrap());
            let object = delete_role_template(configuration, minimal_object.object_id.as_ref().unwrap().as_ref()).await;
            match object {
                Ok(object) => {
                    info!("Deleted role-template `{}`", minimal_object.object_id.as_ref().unwrap());
                    Ok(CreatedObject::RoleTemplate(object))},
                Err(e) => {
                    error!("Failed to delete role-template `{}`: {}", minimal_object.object_id.as_ref().unwrap(), e);
                    Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            }
        }
        ObjectType::ProjectRoleTemplateBinding => {
            let cluster_id = minimal_object.namespace.as_deref().unwrap_or("<no-namespace>");
            info!("Deleting prtb `{}` from cluster `{}`", minimal_object.object_id.as_ref().unwrap(), cluster_id);
            let object = delete_project_role_template_binding(configuration, cluster_id, minimal_object.object_id.as_ref().unwrap().as_ref()).await;
            match object {
                Ok(object) => {
                    info!("Deleted prtb `{}` from cluster `{}`", minimal_object.object_id.as_ref().unwrap(), cluster_id);
                    Ok(CreatedObject::ProjectRoleTemplateBinding(object))
                },
                Err(e) => {
                    error!("Failed to delete prtb `{}` from cluster `{}`: {}", minimal_object.object_id.as_ref().unwrap(), cluster_id, e);
                    Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                }
            }
        }
        _ => panic!("Unsupported object type: {:?}", object_type),
    }
}




pub async fn create_objects(
    configuration: Arc<Configuration>,
    new_files: Vec<(ObjectType, PathBuf)>,
    file_format: FileFormat,
) -> Vec<Result<(PathBuf, CreatedObject), Box<dyn std::error::Error + Send + Sync>>> {
    let mut new_files = new_files;
    let mut results = Vec::with_capacity(new_files.len());
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
                    info!("Creating role-template from file: {}", file_path.display());
                    let role_template = load_object::<RoleTemplate>(&file_path, &format).await?;
                    let rancher_rt = IoCattleManagementv3RoleTemplate::try_from(role_template)?;
                    let created = create_role_template(&config, rancher_rt).await?;
                    info!("Created role-template: {}", created.metadata.as_ref().unwrap().name.as_ref().unwrap());
                    Ok((file_path, CreatedObject::RoleTemplate(created)))
                }));
            }
            ObjectType::Project => {
                handles_projects.push(tokio::spawn(async move {
                    info!("Creating project from file: {}", file_path.display());
                    let project = load_object::<Project>(&file_path, &format).await?;
                    let mut rancher_p = IoCattleManagementv3Project::try_from(project)?;
                    let cluster_name = rancher_p
                        .spec
                        .as_ref()
                        .ok_or("Missing spec")?
                        .cluster_name
                        .clone();
                    // check if the metadata.name exists and if not add metadata.generateName: p- to the project's metadata
                    if rancher_p.metadata.is_none() || rancher_p.metadata.as_ref().unwrap().name.is_none() {
                        let mut metadata = rancher_p.metadata.unwrap_or(IoK8sApimachineryPkgApisMetaV1ObjectMeta::default());
                        metadata.generate_name = Some("p-".to_string());
                        rancher_p.metadata = Some(metadata);
                    }
                    let created = create_project(&config, &cluster_name, rancher_p).await?;
                    let display_name = created
                        .metadata
                        .as_ref()
                        .and_then(|m| m.name.as_deref())
                        .ok_or("Missing metadata.name in created project")?;

                    info!("Created project: {}", display_name);

                    Ok((file_path, CreatedObject::Project(created)))
                }));
            }
            ObjectType::ProjectRoleTemplateBinding => {
                handles_prtbs.push(tokio::spawn(async move {
                    info!("Creating prtb from file: {}", file_path.display());
                    let prtb = load_object::<ProjectRoleTemplateBinding>(&file_path, &format).await?;
                    let display_name = prtb.id.clone();
                    let mut rancher_prtb = IoCattleManagementv3ProjectRoleTemplateBinding::try_from(prtb)?;
                    let project_id = rancher_prtb
                        .metadata
                        .as_ref()
                        .and_then(|m| m.namespace.clone())
                        .ok_or("Missing namespace in metadata")?;

                    if rancher_prtb.metadata.is_none() || rancher_prtb.metadata.as_ref().unwrap().name.is_none() {
                        let mut metadata = rancher_prtb.metadata.unwrap_or(IoK8sApimachineryPkgApisMetaV1ObjectMeta::default());
                        metadata.generate_name = Some("prtb-".to_string());
                        rancher_prtb.metadata = Some(metadata);
                    }

                    const MAX_RETRIES: usize = 5;
                    const RETRY_DELAY: Duration = Duration::from_millis(200);

                    let result = retry_async(
                        "create_project_role_template_binding",
                        MAX_RETRIES,
                        RETRY_DELAY,
                        || {
                            let config = config.clone();
                            let rancher_prtb = rancher_prtb.clone();
                            let project_id = project_id.clone();
                            async move {
                                create_project_role_template_binding(&config, &project_id, rancher_prtb.clone()).await
                            }
                        },
                        |err| matches!(
                            err,
                            Error::ResponseError(resp)
                            if resp.status == StatusCode::NOT_FOUND || resp.status == StatusCode::INTERNAL_SERVER_ERROR
                        ),
                    )
                    .await;

                    match result {
                        Ok(created) => {
                            info!("Created PRTB: {}", display_name);
                            Ok((file_path, CreatedObject::ProjectRoleTemplateBinding(created)))
                        }
                        Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
                    }
                }));
            }
            _ => unreachable!(),
        }
    }

    // for the ok results of the role templates poll them
    let rts = await_handles(handles_role_templates).await;
    let poll_tasks = rts.into_iter().filter_map(|res| {
        match res {
            Ok((path, CreatedObject::RoleTemplate(rt))) => {
                let configuration = configuration.clone();
                let fut = async move {
                    info!("Polling role-template {} for readiness", rt.metadata.as_ref().unwrap().name.as_ref().unwrap());
                    let poll_result = poll_role_template_ready(configuration, &rt).await;
                    match poll_result {
                        Ok(_) => {
                            info!("Role-template {} is ready", rt.metadata.as_ref().unwrap().name.as_ref().unwrap());
                            Ok((path, CreatedObject::RoleTemplate(rt)))
                        }
                        Err(e) => Err(e),
                    }
                };
                Some(fut.boxed())
            }
            other => {
                // Wrap the already-evaluated result into a ready future
                let fut = async move { other }.boxed();
                Some(fut)
            }
        }
    });

    // Run polling with a bounded number of concurrent futures
    let polled_rts: Vec<_> = stream::iter(poll_tasks)
        .buffer_unordered(10) // Adjust concurrency level here
        .collect()
        .await;

    // Now append `polled_rts` to the final results
    results.extend(polled_rts);

    let projects = await_handles(handles_projects).await;

    let poll_tasks = projects.into_iter().filter_map(|res| {
        match res {
            Ok((path, CreatedObject::Project(p))) => {
                let configuration = configuration.clone();
                let fut = async move {
                    info!("Polling project {} for readiness", p.metadata.as_ref().unwrap().name.as_ref().unwrap());
                    let poll_result = poll_project_ready(configuration, &p).await;
                    match poll_result {
                        Ok(_) => {
                            info!("Project {} is ready", p.metadata.as_ref().unwrap().name.as_ref().unwrap());
                            Ok((path, CreatedObject::Project(p)))
                        }
                        Err(e) => Err(e),
                    }
                };
                Some(fut.boxed())
            }
            other => {
                // Wrap the already-evaluated result into a ready future
                let fut = async move { other }.boxed();
                Some(fut)
            }
        }
    });

    // Run polling with a bounded number of concurrent futures
    let polled_projects: Vec<_> = stream::iter(poll_tasks)
        .buffer_unordered(10) // Adjust concurrency level here
        .collect()
        .await;

    // Now append `polled_projects` to the final results
    results.extend(polled_projects);

    results.extend(await_handles(handles_prtbs).await);
    results
}
