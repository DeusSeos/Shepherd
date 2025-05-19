use crate::{poll_project_ready, poll_role_template_ready, retry_async, RoleTemplate};
use crate::diff::compute_cluster_diff;
use crate::models::CreatedObject;
use crate::project::{create_project, update_project};
use crate::prtb::update_project_role_template_binding;
use crate::rt::update_role_template;
use crate::{await_handles, load_configuration, load_configuration_from_rancher, load_object, ObjectType};
use crate::config::RancherClusterConfig;
use crate::file::FileFormat;

use rancher_client::apis::configuration::Configuration;
use rancher_client::apis::Error;
use rancher_client::models::{IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding, IoCattleManagementv3RoleTemplate};
use reqwest::StatusCode;

use futures::{stream, FutureExt, StreamExt};
use tracing::info;


use crate::project::Project;
use crate::prtb::{create_project_role_template_binding, ProjectRoleTemplateBinding};
use crate::rt::create_role_template;


use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub async fn compare_and_update_configurations(
    configuration: Arc<Configuration>,
    config_folder_path: &Path,
    cluster_id: &str,
    file_format: &FileFormat,
) -> Vec<Result<CreatedObject, Box<dyn std::error::Error + Send + Sync>>> {
    // Load the stored configuration
    let stored_config = load_configuration(config_folder_path, &configuration.base_path, cluster_id, file_format).await.unwrap().unwrap();
    let stored_config: RancherClusterConfig = RancherClusterConfig::try_from(stored_config).unwrap();

    // Load the live Rancher configuration
    let live_config = load_configuration_from_rancher(&configuration, cluster_id).await.unwrap();

    // Compute the differences
    let diffs = compute_cluster_diff(&serde_json::to_value(&live_config).unwrap(), &serde_json::to_value(&stored_config).unwrap());
    info!("Found {:#?} differences", diffs);

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
            println!("  → project `{}` in namespace `{}`", object_id, ns);
            let object = update_project(&configuration, &ns, &object_id, diff_value).await;
            match object {
                Ok(object) => Ok(CreatedObject::Project(object)),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }

        }

        ObjectType::RoleTemplate => {
            println!("  → role-template `{}`", object_id);
            let object = update_role_template(&configuration, &object_id, diff_value).await;
            match object {
                Ok(object) => Ok(CreatedObject::RoleTemplate(object)),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }
        }

        ObjectType::ProjectRoleTemplateBinding => {
            let ns = namespace.as_deref().unwrap_or("<no-namespace>");
            println!("  → prtb `{}` in namespace `{}`", object_id, ns);
            let object = update_project_role_template_binding(&configuration, &ns, &object_id, diff_value).await;
            match object {
                Ok(object) => Ok(CreatedObject::ProjectRoleTemplateBinding(object)),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
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
                    let role_template = load_object::<RoleTemplate>(&file_path, &format).await?;
                    let rancher_rt = IoCattleManagementv3RoleTemplate::try_from(role_template)?;
                    let created = create_role_template(&config, rancher_rt).await?;
                    Ok((file_path, CreatedObject::RoleTemplate(created)))
                }));
            }
            ObjectType::Project => {
                handles_projects.push(tokio::spawn(async move {
                    let project = load_object::<Project>(&file_path, &format).await?;
                    let rancher_p = IoCattleManagementv3Project::try_from(project)?;
                    let cluster_name = rancher_p
                        .spec
                        .as_ref()
                        .ok_or("Missing spec")?
                        .cluster_name
                        .clone();
                    let created = create_project(&config, &cluster_name, rancher_p).await?;
                    let display_name = created
                        .metadata
                        .as_ref()
                        .and_then(|m| m.name.as_deref())
                        .ok_or("Missing metadata.name in created project")?;

                    info!("Created project: {}", display_name);

                    // if let Ok(created_project) = poll_project_ready(config.clone(), created.clone()).await {
                    //     println!("Created and verified project: {}", display_name);
                    //     Ok((file_path, CreatedObject::Project(created_project)))
                    // } else {
                    //     Err("Failed to verify project creation".into())
                    // }
                    Ok((file_path, CreatedObject::Project(created)))
                }));
            }
            ObjectType::ProjectRoleTemplateBinding => {
                handles_prtbs.push(tokio::spawn(async move {
                    let prtb = load_object::<ProjectRoleTemplateBinding>(&file_path, &format).await?;
                    let display_name = prtb.id.clone();
                    let rancher_prtb = IoCattleManagementv3ProjectRoleTemplateBinding::try_from(prtb)?;
                    let project_id = rancher_prtb
                        .metadata
                        .as_ref()
                        .and_then(|m| m.namespace.clone())
                        .ok_or("Missing namespace in metadata")?;

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
                    let poll_result = poll_role_template_ready(configuration, &rt).await;
                    match poll_result {
                        Ok(_) => Ok((path, CreatedObject::RoleTemplate(rt))),
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
                    let poll_result = poll_project_ready(configuration, &p).await;
                    match poll_result {
                        Ok(_) => Ok((path, CreatedObject::Project(p))),
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