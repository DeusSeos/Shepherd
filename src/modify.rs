use crate::api::config::RancherClusterConfig;
use crate::traits::RancherResource;
use crate::utils::diff::{calculate_json_patch, compute_cluster_diff};
use crate::utils::file::FileFormat;
use crate::models::{CreatedObject, MinimalObject};
use crate::resources::project::{create_project, find_project, update_project, PROJECT_EXCLUDE_PATHS};
use crate::resources::prtb::{find_project_role_template_binding, update_project_role_template_binding, PRTB_EXCLUDE_PATHS};
use crate::resources::rt::{find_role_template, update_role_template, RT_EXCLUDE_PATHS};
use crate::{
    await_handles, clean_up_value, load_configuration, load_configuration_from_rancher, load_object, ObjectType
};
use crate::{poll_project_ready, poll_role_template_ready, retry_async, RoleTemplate};

use rancher_client::apis::configuration::Configuration;
use rancher_client::apis::management_cattle_io_v3_api::CreateManagementCattleIoV3NamespacedProjectRoleTemplateBindingError;
use rancher_client::models::{
    IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding, IoCattleManagementv3RoleTemplate, IoK8sApimachineryPkgApisMetaV1ObjectMeta
};
use reqwest::StatusCode;

use futures::{stream, FutureExt, StreamExt};
use tracing::{debug, error, info, trace};

use crate::resources::project::Project;
use crate::resources::prtb::{create_project_role_template_binding, ProjectRoleTemplateBinding};

use anyhow::Result;

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
pub async fn compare_and_update_infrastructure(
    configuration: Arc<Configuration>,
    config_folder_path: &Path,
    cluster_id: &str,
    file_format: &FileFormat,
) -> Vec<Result<CreatedObject>> {
    // Load the stored configuration
    let stored_config = load_configuration(
        config_folder_path,
        &configuration.base_path,
        cluster_id,
        file_format,
    )
    .await
    .unwrap()
    .unwrap();
    debug!(
        "Loaded stored configuration for cluster `{}`: {} ",
        cluster_id, stored_config
    );
    let stored_config: RancherClusterConfig =
        RancherClusterConfig::try_from(stored_config).unwrap();

    // Load the live Rancher configuration
    let live_config = load_configuration_from_rancher(&configuration, cluster_id)
        .await
        .unwrap();

    debug!(
        "Loaded live configuration for cluster `{}`: {} ",
        cluster_id, live_config
    );
    let live_config: RancherClusterConfig = RancherClusterConfig::try_from(live_config).unwrap();

    // find the items in the stored config that are not in the live config
    // we need to create them


    // Compute the differences
    let diffs = compute_cluster_diff(
        &serde_json::to_value(&live_config).unwrap(),
        &serde_json::to_value(&stored_config).unwrap(),
    );
    debug!(
        "Generated diffs for cluster `{}`: {:#?} ",
        cluster_id, diffs
    );

    let mut results: Vec<Result<CreatedObject>> = Vec::new();



    // Iterate through the differences and handle them use tokio to do them in parallel
    let mut handles = Vec::with_capacity(diffs.len());
    for ((object_type, object_id, namespace), diff_value) in diffs {
        let handle = tokio::spawn(update_object(
            configuration.clone(),
            object_type,
            object_id,
            namespace,
            diff_value,
        ));
        handles.push(handle);
    }
    for result in stream::iter(handles)
        .buffer_unordered(8)
        .collect::<Vec<_>>()
        .await
    {
        match result {
            Ok(object) => results.push(object),
            Err(e) => results.push(Err(e.into())),
        }
    }

    results
}


    /// Updates objects in the cluster with the given desired state.
    ///
    /// # Arguments
    ///
    /// * `configuration` - The configuration object
    /// * `updated_files` - A vector of tuples containing the object type and the path to the file
    ///     containing the desired state
    ///
    /// # Returns
    ///
    /// * `Vec<Result<CreatedObject>>` - A vector of results for each object, containing the created
    ///     object or an error if the update failed
pub async fn update_objects(
    configuration: Arc<Configuration>,
    updated_files: Vec<(ObjectType, PathBuf)>,
) -> Vec<Result<CreatedObject>> {
    let mut handles = Vec::with_capacity(updated_files.len());
    for (object_type, file_path) in updated_files {

        let mut desired_state;
        let object_id: String;
        let mut namespace: String = String::new();

        match object_type {
            ObjectType::Project => {
                let object = load_object::<Project>(&file_path).await.unwrap();
                object_id = object.id.clone().unwrap();
                namespace = object.namespace.clone();
                // convert object to IoCattleManagementv3Project
                let object = IoCattleManagementv3Project::try_from(object).unwrap();
                desired_state = serde_json::to_value(object).unwrap();
            }
            ObjectType::ProjectRoleTemplateBinding => {
                let object = load_object::<ProjectRoleTemplateBinding>(&file_path).await.unwrap();
                object_id = object.id.clone();
                namespace = object.namespace.clone();
                // convert object to IoCattleManagementv3ProjectRoleTemplateBinding
                let object = IoCattleManagementv3ProjectRoleTemplateBinding::try_from(object).unwrap();
                desired_state = serde_json::to_value(object).unwrap();
            }
            ObjectType::RoleTemplate => {
                let object = load_object::<RoleTemplate>(&file_path).await.unwrap();
                object_id = object.id.clone();
                // convert object to IoCattleManagementv3RoleTemplate
                let object = IoCattleManagementv3RoleTemplate::try_from(object).unwrap();
                desired_state = serde_json::to_value(object).unwrap();
            }
            _ => 
            {
                // skip unsupported object types
                error!("Unsupported object type: {:?}", object_type);
                continue;
            },
        }

        // create a handle to update the object
        // it will get the object from the Rancher API
        // calculate the diff
        // update the object
        let handle = tokio::spawn({
            let configuration = configuration.clone();
            let object_type = object_type;
            let object_id = object_id.clone();
            let namespace = namespace.clone();
            let mut diff = None;
            
            // find the object
            match object_type {
                ObjectType::Project => {
                    let project: IoCattleManagementv3Project = find_project(&configuration, &namespace, &object_id, None).await.unwrap();
                    let mut current_state = serde_json::to_value(&project).unwrap();
                    trace!("Current state: {:#?}", current_state);
                    trace!("Desired state: {:#?}", desired_state);
                    // clean both states
                    clean_up_value(&mut current_state, PROJECT_EXCLUDE_PATHS);
                    clean_up_value(&mut desired_state, PROJECT_EXCLUDE_PATHS);
                    diff = calculate_json_patch::<IoCattleManagementv3Project>(&current_state, &desired_state);
                    trace!("Diff: {:#?}", diff);
                }
                ObjectType::ProjectRoleTemplateBinding => {
                    let prtb = find_project_role_template_binding(&configuration, &namespace, &object_id).await.unwrap();
                    let mut current_state = serde_json::to_value(&prtb).unwrap();
                    clean_up_value(&mut current_state, PRTB_EXCLUDE_PATHS);
                    clean_up_value(&mut desired_state, PRTB_EXCLUDE_PATHS);
                    diff = calculate_json_patch::<IoCattleManagementv3ProjectRoleTemplateBinding>(&current_state, &desired_state);
                }
                ObjectType::RoleTemplate => {
                    let rt = find_role_template(&configuration, &object_id, None).await.unwrap();
                    let mut current_state = serde_json::to_value(&rt).unwrap();
                    clean_up_value(&mut current_state, RT_EXCLUDE_PATHS);
                    clean_up_value(&mut desired_state, RT_EXCLUDE_PATHS);
                    diff = calculate_json_patch::<IoCattleManagementv3RoleTemplate>(&current_state, &desired_state);
                }
                _ => 
                {
                    // skip unsupported object types
                    error!("Unsupported object type: {:?}", object_type);
                    
                },
            }

            async move {
                if diff.is_none() {
                    error!("No diff found for object type: {:?}", object_type);
                    return Err(anyhow::anyhow!("No diff found for object type: {:?}", object_type));
                }
                update_object(configuration, object_type, object_id,Some(namespace), diff.unwrap()).await
            }
            
        });

        handles.push(handle);
        
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(object) => results.push(object),
            Err(join_err) => results.push(Err(anyhow::anyhow!(join_err))),
        }
    }
    results


}

async fn update_object(
    configuration: Arc<Configuration>,
    object_type: ObjectType,
    object_id: String,
    namespace: Option<String>,
    diff_value: Value,
) -> Result<CreatedObject> {
    match object_type {
        ObjectType::Project => {
            let ns = namespace.as_deref().unwrap_or("<no-namespace>");
            debug!(
                "Updating project `{}` in cluster `{} with diff: {:#?}`",
                object_id, ns, diff_value
            );
            let object = update_project(&configuration, ns, &object_id, diff_value).await;
            match object {
                Ok(object) => {
                    info!(
                        "Updated project `{}` ({}) in cluster `{}`",
                        object_id,
                        object.spec.as_ref().unwrap().display_name,
                        ns
                    );
                    Ok(CreatedObject::Project(object))
                }
                Err(e) => Err(e),
            }
        }

        ObjectType::RoleTemplate => {
            info!("Update role-template `{}`", object_id);
            debug!(
                "Update role-template `{}` with diff: {:#?} ",
                object_id, diff_value
            );
            let object = update_role_template(&configuration, &object_id, diff_value).await;
            match object {
                Ok(object) => Ok(CreatedObject::RoleTemplate(object)),
                Err(e) => Err(e),
            }
        }

        ObjectType::ProjectRoleTemplateBinding => {
            let ns = namespace.as_deref().unwrap_or("<no-namespace>");
            info!("Updated prtb `{}` in namespace `{}`", object_id, ns);
            debug!(
                "Updated prtb `{}` in namespace `{}` with diff: {:#?} ",
                object_id, ns, diff_value
            );
            let object =
                update_project_role_template_binding(&configuration, ns, &object_id, diff_value)
                    .await;
            match object {
                Ok(object) => Ok(CreatedObject::ProjectRoleTemplateBinding(object)),
                Err(e) => Err(e),
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
) -> Vec<Result<CreatedObject>> {
    let mut results = Vec::with_capacity(deleted_files.len());

    // sort the deleted files by object type backwards
    let mut deleted_files = deleted_files;
    deleted_files.sort_by(|a, b| b.0.priority().cmp(&a.0.priority()));

    for (object_type, minimal_object) in deleted_files {
        match delete_object(&configuration, &object_type, &minimal_object).await {
            Ok(object) => {
                trace!("Deleted object: {:#?}", minimal_object);
                results.push(Ok(object))
            }
            Err(e) => {
                error!("Error deleting {:?} object: {}", minimal_object, e);
                results.push(Err(e))
            }
        }
    }
    results
}

/// Deletes an object from the cluster
async fn delete_object(
    configuration: &Arc<Configuration>,
    object_type: &ObjectType,
    minimal_object: &MinimalObject,
) -> Result<CreatedObject> {
    let name = minimal_object.object_id.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Object ID is required for deletion"))?;
    
    let namespace = minimal_object.namespace.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Namespace is required for deletion"))?;
    
    match object_type {
        ObjectType::Project => {
            Project::delete(configuration, name, namespace).await
        },
        ObjectType::ProjectRoleTemplateBinding => {
            ProjectRoleTemplateBinding::delete(configuration, name, namespace).await
            // ProjectRoleTemplateBinding::delete(configuration, name, namespace).await?;
        },
        ObjectType::RoleTemplate => {
            RoleTemplate::delete(configuration, name, namespace).await
            // RoleTemplate::delete(configuration, name, namespace).await?;
        },
        _ => Err(anyhow::anyhow!("Unsupported object type: {:?}", object_type)),
    }
    
}


/// Creates objects from files in the given directory
///
/// # Arguments
/// * `configuration` - The configuration object
/// * `new_files` - A vector of tuples containing the object type and the path to the file
/// * `file_format` - The format of the files
///
/// # Returns
/// * `Vec<Result<(PathBuf, CreatedObject)>>`
pub async fn create_objects(
    configuration: Arc<Configuration>,
    new_files: Vec<(ObjectType, PathBuf)>,
    concurrency: usize, max_retries: usize, retry_delay: Duration
) -> Vec<Result<(PathBuf, CreatedObject)>> {
    // Mutable vector for file processing results
    let mut new_files = new_files;
    let mut results = Vec::with_capacity(new_files.len());

    // Sort the files based on object type priority
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

    // Create vectors to store tasks for different object types
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

    // Iterate through each file and create tasks based on object type
    for (object_type, file_path) in new_files {
        let config = configuration.clone();
        match object_type {
            ObjectType::RoleTemplate => {
                // Spawn task to create role template
                handles_role_templates.push(tokio::spawn(async move {
                    info!(path = %file_path.display(), "Creating role-template from file");
                    let role_template = load_object::<RoleTemplate>(&file_path).await?;
                    let created = role_template.create(&config).await?;
                    match created {
                        CreatedObject::RoleTemplate(ref object) => {
                            info!( "Created role-template: {}", object.metadata.as_ref().unwrap().name.as_ref().unwrap() );
                            Ok((file_path, created))
                        }
                        other => {
                            error!( "Failed to create role-template: {:#?}", other );
                            Err(anyhow::anyhow!("Failed to create role-template"))
                        },
                    }
                }));
            }
            ObjectType::Project => {
                // Spawn task to create project
                handles_projects.push(tokio::spawn(async move {
                    info!(path = %file_path.display(), "Creating project from file");
                    let project = load_object::<Project>(&file_path).await?;
                    let mut rancher_p = IoCattleManagementv3Project::try_from(project)?;
                    let cluster_name = rancher_p
                            .spec
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("Missing spec"))?
                            .cluster_name
                            .clone();

                    // Ensure project has a name or generate one
                    if rancher_p.metadata.is_none()
                        || rancher_p.metadata.as_ref().unwrap().name.is_none()
                    {
                        let mut metadata = rancher_p
                            .metadata
                            .unwrap_or(IoK8sApimachineryPkgApisMetaV1ObjectMeta::default());
                        metadata.generate_name = Some("p-".to_string());
                        rancher_p.metadata = Some(metadata);
                    }
                    let created = create_project(&config, &cluster_name, rancher_p).await?;
                    let display_name = created
                        .metadata
                        .as_ref()
                        .and_then(|m| m.name.as_deref())
                        .ok_or_else(|| anyhow::anyhow!("Missing metadata.name in created project"))?;

                    info!("Created project: {}", display_name);
                    Ok((file_path, CreatedObject::Project(created)))
                }));
            }
            ObjectType::ProjectRoleTemplateBinding => {
                // Collect files for ProjectRoleTemplateBinding
                handles_prtbs.push(file_path);
            }
            _ => unreachable!(),
        }
    }

    // Process role template tasks and poll for readiness
    let rts = await_handles(handles_role_templates).await;
    let poll_tasks = rts.into_iter().filter_map(|res| {
        match res {
            Ok((path, CreatedObject::RoleTemplate(rt))) => {
                let configuration = configuration.clone();
                let fut = async move {
                    info!(
                        "Polling role-template {} for readiness",
                        rt.metadata.as_ref().unwrap().name.as_ref().unwrap()
                    );
                    let poll_result = poll_role_template_ready(configuration, &rt).await;
                    match poll_result {
                        Ok(_) => {
                            info!(
                                "Role-template {} is ready",
                                rt.metadata.as_ref().unwrap().name.as_ref().unwrap()
                            );
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
        .buffer_unordered(concurrency) // Adjust concurrency level here
        .collect()
        .await;

    // Append `polled_rts` to the final results
    results.extend(polled_rts);

    // Process project tasks and poll for readiness
    let projects = await_handles(handles_projects).await;
    let poll_tasks = projects.into_iter().filter_map(|res| {
        match res {
            Ok((path, CreatedObject::Project(p))) => {
                let configuration = configuration.clone();
                let fut = async move {
                    info!(
                        "Polling project {} for readiness",
                        p.metadata.as_ref().unwrap().name.as_ref().unwrap()
                    );
                    let poll_result = poll_project_ready(configuration, &p).await;
                    match poll_result {
                        Ok(_) => {
                            info!(
                                "Project {} is ready",
                                p.metadata.as_ref().unwrap().name.as_ref().unwrap()
                            );
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
        .buffer_unordered(concurrency) // Adjust concurrency level here
        .collect()
        .await;

    // Append `polled_projects` to the final results
    results.extend(polled_projects);

    // Process ProjectRoleTemplateBinding files
    let mut prtb_handles = Vec::with_capacity(handles_prtbs.len());
    for file_path in handles_prtbs {
        let config = configuration.clone();
        prtb_handles.push(tokio::spawn(async move {
            info!(path = %file_path.display(), "Creating project-role-template-binding from file");
            let prtb = load_object::<ProjectRoleTemplateBinding>(&file_path).await?;
            let display_name = prtb.id.clone();
            let mut rancher_prtb = IoCattleManagementv3ProjectRoleTemplateBinding::try_from(prtb)?;
            let project_id = rancher_prtb
                .metadata
                .as_ref()
                .and_then(|m| m.namespace.clone())
                .ok_or_else(|| anyhow::anyhow!("Missing metadata.namespace in PRTB"))?;

            // Ensure PRTB has a name or generate one
            if rancher_prtb.metadata.is_none() || rancher_prtb.metadata.as_ref().unwrap().name.is_none() {
                let mut metadata = rancher_prtb.metadata.unwrap_or(IoK8sApimachineryPkgApisMetaV1ObjectMeta::default());
                metadata.generate_name = Some("prtb-".to_string());
                rancher_prtb.metadata = Some(metadata);
            }

            // Retry logic for creating PRTB
            

let result = retry_async(
    "create_project_role_template_binding",
    max_retries,
    retry_delay,
    || {
        let config = config.clone();
        let rancher_prtb = rancher_prtb.clone();
        let project_id = project_id.clone();
        async move {
            create_project_role_template_binding(&config, &project_id, rancher_prtb.clone()).await
        }
    },
    |err| {
        // Try to downcast the anyhow::Error to the specific rancher error type
        let rancher_err = err.downcast_ref::<rancher_client::apis::Error<CreateManagementCattleIoV3NamespacedProjectRoleTemplateBindingError>>();
        
        if let Some(rancher_client::apis::Error::ResponseError(resp)) = rancher_err {
            resp.status == StatusCode::NOT_FOUND || resp.status == StatusCode::INTERNAL_SERVER_ERROR
        } else {
            // Check for other error types if needed
            false
        }
    },
)
.await;

match result {
    Ok(created) => {
        info!("Created PRTB: {}", display_name);
        Ok((file_path, CreatedObject::ProjectRoleTemplateBinding(created)))
    }
    Err(e) => Err(e), // Already an anyhow::Error
}
        }));
    }

    // Append the results of PRTB tasks
    results.extend(await_handles(prtb_handles).await);
    results
}
