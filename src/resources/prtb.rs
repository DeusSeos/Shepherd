use serde::{Deserialize, Serialize};

use crate::{utils::logging::log_api_error, models::ResourceVersionMatch};
use anyhow::{Context as anyhow_context, Result};

use reqwest::StatusCode;

use rancher_client::{
    apis::{
        configuration::Configuration,
        management_cattle_io_v3_api::{
            create_management_cattle_io_v3_namespaced_project_role_template_binding,
            delete_management_cattle_io_v3_namespaced_project_role_template_binding,
            list_management_cattle_io_v3_namespaced_project_role_template_binding,
            list_management_cattle_io_v3_project_role_template_binding_for_all_namespaces,
            patch_management_cattle_io_v3_namespaced_project_role_template_binding, ListManagementCattleIoV3NamespacedProjectRoleTemplateBindingError
        },
        Error, ResponseContent,
    },
    models::{
        IoCattleManagementv3ProjectRoleTemplateBinding,
        IoCattleManagementv3ProjectRoleTemplateBindingList,
        IoK8sApimachineryPkgApisMetaV1ObjectMeta, IoK8sApimachineryPkgApisMetaV1Patch, IoK8sApimachineryPkgApisMetaV1Status,
    },
};
use serde_json::Value;
use tracing::{debug, error, info, trace};

pub const PRTB_EXCLUDE_PATHS: &[&str] = &[
    "metadata.creationTimestamp",
    "metadata.finalizers",
    "metadata.generateName",
    "metadata.generation",
    "metadata.managedFields",
    "metadata.resourceVersion",
    "metadata.selfLink",
    "metadata.uid",
];

#[async_backtrace::framed]
pub async fn get_project_role_template_bindings(
    configuration: &Configuration,
    cluster_id: &str,
    field_selector: Option<&str>,
    label_selector: Option<&str>,
    limit: Option<i32>,
    resource_version: Option<&str>,
    resource_version_match: Option<&str>,
    continue_: Option<&str>,
) -> Result<IoCattleManagementv3ProjectRoleTemplateBindingList> {
    debug!("Getting project role template bindings for cluster: {}", cluster_id);

    let api_result = list_management_cattle_io_v3_namespaced_project_role_template_binding(
        configuration,
        cluster_id,
        None,
        None,
        continue_,
        field_selector,
        label_selector,
        limit,
        resource_version,
        resource_version_match,
        None,
        None,
        None,
    )
    .await
    .context(format!(
        "Failed to get project role template bindings for cluster: {}",
        cluster_id
    ));

    match api_result {
        Err(e) => {
            log_api_error("get_project_role_template_bindings", &e);
            Err(anyhow::anyhow!(e))
        }
        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");
            
            match response_content.status {
                StatusCode::OK => {
                    match serde_json::from_str::<IoCattleManagementv3ProjectRoleTemplateBindingList>(&response_content.content) {
                        Ok(data) => {
                            debug!("Successfully retrieved {} project role template bindings for cluster: {}", data.items.len(), cluster_id);
                            Ok(data)
                        },
                        Err(deserialize_err) => {
                            let err = anyhow::anyhow!("Failed to deserialize project role template bindings response: {}", deserialize_err);
                            log_api_error("get_project_role_template_bindings:deserialize", &err);
                            Err(err)
                        }
                    }
                },
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized to get project role template bindings for cluster: {}. Response: {}",
                        cluster_id,
                        response_content.content
                    );
                    log_api_error("get_project_role_template_bindings:unauthorized", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden to get project role template bindings for cluster: {}. Response: {}",
                        cluster_id,
                        response_content.content
                    );
                    log_api_error("get_project_role_template_bindings:forbidden", &err);
                    Err(err)
                }
                status => {
                    let err = match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when getting project role template bindings for cluster: {}: {}", 
                                status, 
                                cluster_id,
                                serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when getting project role template bindings for cluster: {}: {}", 
                                status, 
                                cluster_id,
                                response_content.content
                            )
                        }
                    };
                    log_api_error("get_project_role_template_bindings:unexpected_status", &err);
                    Err(err)
                }
            }
        }
    }
}

/// Get all project role template bindings from a namespace using the provided configuration
///
/// # Arguments
///
/// * `configuration` - The configuration to use for the request
/// * `project_id` - The ID of the project (namespace) to get the role template bindings for
/// * `field_selector` - If specified, selects only the specified fields of the bindings
/// * `label_selector` - If specified, selects only the bindings with the specified labels
/// * `limit` - If specified, limits the number of bindings returned
/// * `resource_version` - If specified, only returns bindings with a resource version greater than the specified version
/// * `resource_version_match` - If specified, only returns bindings with a resource version that matches the specified version
/// * `continue_` - If specified, continues the listing from the last binding returned in the previous response
///
/// # Returns
///
/// * `IoCattleManagementv3ProjectRoleTemplateBindingList` - The list of project role template bindings
///
/// # Errors
///
/// * `Error<ListManagementCattleIoV3NamespacedProjectRoleTemplateBindingError>` - The error that occurred while trying to get the bindings
#[async_backtrace::framed]
pub async fn get_namespaced_project_role_template_bindings(
    configuration: &Configuration,
    project_id: &str,
    field_selector: Option<&str>,
    label_selector: Option<&str>,
    limit: Option<i32>,
    resource_version: Option<&str>,
    resource_version_match: Option<&str>,
    continue_: Option<&str>,
) -> Result<
    IoCattleManagementv3ProjectRoleTemplateBindingList,
    Error<ListManagementCattleIoV3NamespacedProjectRoleTemplateBindingError>,
> {
    debug!(
        "Fetching project role template bindings for project_id: {}, with filters - field_selector: {:?}, label_selector: {:?}, limit: {:?}, resource_version: {:?}, resource_version_match: {:?}, continue: {:?}",
        project_id, field_selector, label_selector, limit, resource_version, resource_version_match, continue_
    );

    let result = list_management_cattle_io_v3_namespaced_project_role_template_binding(
        configuration,
        project_id,
        None,
        None,
        continue_,
        field_selector,
        label_selector,
        limit,
        resource_version,
        resource_version_match,
        None,
        None,
        None,
    )
    .await;

    match result {
        Err(e) => {
            error!("Failed to fetch project role template bindings: {}", e);
            Err(e)
        }
        Ok(response_content) => {
            trace!("Received response: {:?}", response_content);

            match response_content.status {
                StatusCode::OK => match serde_json::from_str(&response_content.content) {
                    Ok(data) => {
                        debug!("Successfully deserialized response content");
                        Ok(data)
                    }
                    Err(deserialize_err) => {
                        error!("Deserialization error: {}", deserialize_err);
                        Err(Error::Serde(deserialize_err))
                    }
                },
                StatusCode::NOT_FOUND => {
                    error!("The project role template bindings were not found");
                    Err(Error::ResponseError(ResponseContent {
                        status: response_content.status,
                        content: response_content.content,
                        entity: None,
                    }))
                }
                StatusCode::UNAUTHORIZED => {
                    error!("You are not authorized to access the project role template bindings");
                    Err(Error::ResponseError(ResponseContent {
                        status: response_content.status,
                        content: response_content.content,
                        entity: None,
                    }))
                }
                StatusCode::FORBIDDEN => {
                    error!("You do not have permission to access the project role template bindings");
                    Err(Error::ResponseError(ResponseContent {
                        status: response_content.status,
                        content: response_content.content,
                        entity: None,
                    }))
                }
                _ => match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                    Ok(unknown_data) => {
                        error!("Received unknown response status: {}", response_content.status);
                        Err(Error::ResponseError(ResponseContent {
                            status: response_content.status,
                            content: response_content.content,
                            entity: Some(ListManagementCattleIoV3NamespacedProjectRoleTemplateBindingError::UnknownValue(
                                unknown_data,
                            )),
                        }))
                    }
                    Err(deserialize_err) => {
                        error!("Deserialization error for unknown response: {}", deserialize_err);
                        Err(Error::Serde(deserialize_err))
                    }
                },
            }
        }
    }
}

/// Update a project role template binding
///
/// # Arguments
///
/// * `configuration` - The configuration to use for the request
/// * `cluster_id` - The cluster ID
/// * `prtb_id` - The project role template binding ID
/// * `patch_value` - The JSON patch to apply
/// # Returns
///
/// * `IoCattleManagementv3ProjectRoleTemplateBinding` - The updated project role template binding
/// # Errors
///
/// * `anyhow::Error` - The error that occurred while trying to update the project role template binding
///
#[async_backtrace::framed]
pub async fn update_project_role_template_binding(
    configuration: &Configuration,
    cluster_id: &str,
    prtb_id: &str,
    patch_value: Value,
) -> Result<IoCattleManagementv3ProjectRoleTemplateBinding> {
    info!("Patching project role template binding with ID: {} in cluster: {}", prtb_id, cluster_id);

    let patch_array = match patch_value {
        Value::Array(arr) => arr,
        Value::Null => {
            let err = anyhow::anyhow!("Expected patch to serialize to a JSON array, but got null");
            log_api_error("update_project_role_template_binding:invalid_patch", &err);
            return Err(err);
        }
        _ => {
            let err = anyhow::anyhow!(
                "Expected patch to serialize to a JSON array, but got: {:?}",
                patch_value
            );
            log_api_error("update_project_role_template_binding:invalid_patch", &err);
            return Err(err);
        }
    };

    let k8s_patch = IoK8sApimachineryPkgApisMetaV1Patch::Array(patch_array);

    let result = patch_management_cattle_io_v3_namespaced_project_role_template_binding(
        configuration,
        cluster_id,
        prtb_id,
        Some(k8s_patch),
        None,
        None,
        None,
        None,
        None
    )
    .await
    .context(format!(
        "Failed to update project role template binding with ID: {} in cluster: {}",
        prtb_id, cluster_id
    ));

    match result {
        Err(e) => {
            log_api_error("update_project_role_template_binding", &e);
            Err(anyhow::anyhow!(e))
        }
        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");

            match response_content.status {
                StatusCode::OK => {
                    match serde_json::from_str::<IoCattleManagementv3ProjectRoleTemplateBinding>(&response_content.content) {
                        Ok(data) => {
                            info!("Successfully updated project role template binding with ID: {}", prtb_id);
                            Ok(data)
                        }
                        Err(deserialize_err) => {
                            let err = anyhow::anyhow!(
                                "Failed to deserialize project role template binding update response: {}",
                                deserialize_err
                            );
                            log_api_error("update_project_role_template_binding:deserialize", &err);
                            Err(err)
                        }
                    }
                }
                StatusCode::NOT_FOUND => {
                    let err = anyhow::anyhow!(
                        "Project role template binding with ID: {} not found for update. Response: {}",
                        prtb_id,
                        response_content.content
                    );
                    log_api_error("update_project_role_template_binding:not_found", &err);
                    Err(err)
                }
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized to update project role template binding with ID: {}. Response: {}",
                        prtb_id,
                        response_content.content
                    );
                    log_api_error("update_project_role_template_binding:unauthorized", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden to update project role template binding with ID: {}. Response: {}",
                        prtb_id,
                        response_content.content
                    );
                    log_api_error("update_project_role_template_binding:forbidden", &err);
                    Err(err)
                }
                StatusCode::CONFLICT => {
                    let err = anyhow::anyhow!(
                        "Conflict when updating project role template binding with ID: {}. Response: {}",
                        prtb_id,
                        response_content.content
                    );
                    log_api_error("update_project_role_template_binding:conflict", &err);
                    Err(err)
                }
                status => {
                    let err = match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when updating project role template binding with ID: {}: {}", 
                                status, 
                                prtb_id,
                                serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when updating project role template binding with ID: {}: {}", 
                                status, 
                                prtb_id,
                                response_content.content
                            )
                        }
                    };
                    log_api_error("update_project_role_template_binding:unexpected_status", &err);
                    Err(err)
                }
            }
        }
    }
}


/// Create a project role template binding
///
/// # Arguments
///
/// * `configuration` - The configuration to use for the request
/// * `project_id` - The cluster ID
/// * `body` - The project role template binding to create
/// # Returns
///
/// * `IoCattleManagementv3ProjectRoleTemplateBinding` - The created project role template binding
/// # Errors
///
/// * `anyhow::Error` - The error that occurred while trying to create the project role template binding
///
#[async_backtrace::framed]
pub async fn create_project_role_template_binding(
    configuration: &Configuration,
    project_id: &str,
    body: IoCattleManagementv3ProjectRoleTemplateBinding,
) -> Result<IoCattleManagementv3ProjectRoleTemplateBinding> {
    let prtb_id = body.metadata.as_ref().unwrap().name.clone().unwrap_or_default();
    
    info!("Creating project role template binding with ID: {} for project: {}", 
          prtb_id, project_id);

    let result = create_management_cattle_io_v3_namespaced_project_role_template_binding(
        configuration,
        project_id,
        body,
        None,
        None,
        None,
        None,
    )
    .await
    .context(format!(
        "Failed to create project role template binding with ID: {} for project: {}",
        prtb_id, project_id
    ));

    match result {
        Err(e) => {
            log_api_error("create_project_role_template_binding", &e);
            Err(anyhow::anyhow!(e))
        }
        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");

            match response_content.status {
                StatusCode::CREATED | StatusCode::OK => {
                    match serde_json::from_str::<IoCattleManagementv3ProjectRoleTemplateBinding>(&response_content.content) {
                        Ok(data) => {
                            info!("Successfully created project role template binding with ID: {} for project: {}", 
                                  prtb_id, project_id);
                            Ok(data)
                        }
                        Err(deserialize_err) => {
                            let err = anyhow::anyhow!(
                                "Failed to deserialize project role template binding creation response: {}",
                                deserialize_err
                            );
                            log_api_error("create_project_role_template_binding:deserialize", &err);
                            Err(err)
                        }
                    }
                }
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized to create project role template binding for project: {}. Response: {}",
                        project_id,
                        response_content.content
                    );
                    log_api_error("create_project_role_template_binding:unauthorized", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden to create project role template binding for project: {}. Response: {}",
                        project_id,
                        response_content.content
                    );
                    log_api_error("create_project_role_template_binding:forbidden", &err);
                    Err(err)
                }
                StatusCode::CONFLICT => {
                    let err = anyhow::anyhow!(
                        "Conflict when creating project role template binding with ID: {} for project: {}. Response: {}",
                        prtb_id,
                        project_id,
                        response_content.content
                    );
                    log_api_error("create_project_role_template_binding:conflict", &err);
                    Err(err)
                }
                status => {
                    let err = match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when creating project role template binding for project: {}: {}", 
                                status, 
                                project_id,
                                serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when creating project role template binding for project: {}: {}", 
                                status, 
                                project_id,
                                response_content.content
                            )
                        }
                    };
                    log_api_error("create_project_role_template_binding:unexpected_status", &err);
                    Err(err)
                }
            }
        }
    }
}


/// Delete a project role template binding
///
/// # Arguments
///
/// * `configuration` - The configuration to use for the request
/// * `project_id` - The project ID
/// * `prtb_id` - The project role template binding ID
/// # Returns
///
/// * `IoK8sApimachineryPkgApisMetaV1Status` - The status of the deletion
/// # Errors
///
/// * `anyhow::Error` - The error that occurred while trying to delete the project role template binding
///
#[async_backtrace::framed]
pub async fn delete_project_role_template_binding(
    configuration: &Configuration,
    project_id: &str,
    prtb_id: &str,
) -> Result<IoK8sApimachineryPkgApisMetaV1Status> {
    info!("Deleting project role template binding with ID: {} in project: {}", prtb_id, project_id);

    let result = delete_management_cattle_io_v3_namespaced_project_role_template_binding(
        configuration,
        prtb_id,
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
        "Failed to delete project role template binding with ID: {} in project: {}",
        prtb_id, project_id
    ));

    match result {
        Err(e) => {
            log_api_error("delete_project_role_template_binding", &e);
            Err(anyhow::anyhow!(e))
        }
        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");

            match response_content.status {
                StatusCode::OK => {
                    match serde_json::from_str::<IoK8sApimachineryPkgApisMetaV1Status>(&response_content.content) {
                        Ok(data) => {
                            info!("Successfully deleted project role template binding with ID: {}", prtb_id);
                            Ok(data)
                        }
                        Err(deserialize_err) => {
                            let err = anyhow::anyhow!(
                                "Failed to deserialize project role template binding deletion response: {}",
                                deserialize_err
                            );
                            log_api_error("delete_project_role_template_binding:deserialize", &err);
                            Err(err)
                        }
                    }
                }
                StatusCode::NOT_FOUND => {
                    let err = anyhow::anyhow!(
                        "Project role template binding with ID: {} not found for deletion. Response: {}",
                        prtb_id,
                        response_content.content
                    );
                    log_api_error("delete_project_role_template_binding:not_found", &err);
                    Err(err)
                }
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized to delete project role template binding with ID: {}. Response: {}",
                        prtb_id,
                        response_content.content
                    );
                    log_api_error("delete_project_role_template_binding:unauthorized", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden to delete project role template binding with ID: {}. Response: {}",
                        prtb_id,
                        response_content.content
                    );
                    log_api_error("delete_project_role_template_binding:forbidden", &err);
                    Err(err)
                }
                StatusCode::CONFLICT => {
                    let err = anyhow::anyhow!(
                        "Conflict when deleting project role template binding with ID: {}. Response: {}",
                        prtb_id,
                        response_content.content
                    );
                    log_api_error("delete_project_role_template_binding:conflict", &err);
                    Err(err)
                }
                status => {
                    let err = match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when deleting project role template binding with ID: {}: {}", 
                                status, 
                                prtb_id,
                                serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when deleting project role template binding with ID: {}: {}", 
                                status, 
                                prtb_id,
                                response_content.content
                            )
                        }
                    };
                    log_api_error("delete_project_role_template_binding:unexpected_status", &err);
                    Err(err)
                }
            }
        }
    }
}



/// Get all project role template bindings for all projects
///
/// # Arguments
///
/// * `configuration` - The configuration to use for the request
/// * `field_selector` - The field selector to use for the request
/// * `label_selector` - The label selector to use for the request
/// * `limit` - The limit to use for the request
/// * `resource_version` - The resource version to use for the request
/// * `resource_version_match` - The resource version match to use for the request
/// * `continue_` - The continue token to use for the request
/// # Returns
///
/// * `IoCattleManagementv3ProjectRoleTemplateBindingList` - The list of project role template bindings
/// # Errors
///
/// * `anyhow::Error` - The error that occurred while trying to get the project role template bindings
///
#[async_backtrace::framed]
pub async fn get_all_project_role_template_bindings(
    configuration: &Configuration,
    field_selector: Option<&str>,
    label_selector: Option<&str>,
    limit: Option<i32>,
    resource_version: Option<&str>,
    resource_version_match: Option<ResourceVersionMatch>,
    continue_: Option<&str>,
) -> Result<IoCattleManagementv3ProjectRoleTemplateBindingList> {
    debug!("Getting all project role template bindings");

    let api_result = list_management_cattle_io_v3_project_role_template_binding_for_all_namespaces(
        configuration,
        None,
        continue_,
        field_selector,
        label_selector,
        limit,
        resource_version,
        resource_version_match.as_ref().map(|rvm| rvm.as_str()),
        None,
        None,
        None,
        None
    )
    .await
    .context("Failed to get all project role template bindings");

    match api_result {
        Err(e) => {
            log_api_error("get_all_project_role_template_bindings", &e);
            Err(anyhow::anyhow!(e))
        }
        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");
            
            match response_content.status {
                StatusCode::OK => {
                    match serde_json::from_str::<IoCattleManagementv3ProjectRoleTemplateBindingList>(&response_content.content) {
                        Ok(data) => {
                            debug!("Successfully retrieved {} project role template bindings", data.items.len());
                            Ok(data)
                        },
                                    Err(deserialize_err) => {
                            let err = anyhow::anyhow!("Failed to deserialize project role template bindings response: {}", deserialize_err);
                            log_api_error("get_all_project_role_template_bindings:deserialize", &err);
                            Err(err)
                        }
                    }
                },
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized to get all project role template bindings. Response: {}",
                        response_content.content
                    );
                    log_api_error("get_all_project_role_template_bindings:unauthorized", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden to get all project role template bindings. Response: {}",
                        response_content.content
                    );
                    log_api_error("get_all_project_role_template_bindings:forbidden", &err);
                    Err(err)
                }
                status => {
                    let err = match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when getting all project role template bindings: {}", 
                                status, 
                                serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when getting all project role template bindings: {}", 
                                status, 
                                response_content.content
                            )
                        }
                    };
                    log_api_error("get_all_project_role_template_bindings:unexpected_status", &err);
                    Err(err)
                }
            }
        }
    }
}



#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProjectRoleTemplateBinding {
    // annotations: Option<std::collections::HashMap<String, String>>,
    /// Annotations applied to the project role template binding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_principal_name: Option<String>,
    /// The name of the project role template binding (typically the Kubernetes metadata.name).
    pub id: String,

    /// Labels applied to the project role template binding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<std::collections::HashMap<String, String>>,

    /// the project (namespace) the project role template exists in
    pub namespace: String,

    /// The name of the project the project role template is bound to (cluster-id:project-id)
    pub project_name: String,

    pub role_template_name: String,

    /// An opaque value that represents the internal version of this object that can be used by clients to determine when objects have changed. May be used for optimistic concurrency, change detection, and the watch operation on a resource or set of resources. Clients must treat these values as opaque and passed unmodified back to the server. They may only be valid for a particular resource or set of resources.  Populated by the system. Read-only. Value must be treated as opaque by clients and . More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#concurrency-control-and-consistency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account: Option<String>,

    /// The UID of the project. This cannot be changed. Rancher will set this value when the project is created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_principal_name: Option<String>,
}

impl ProjectRoleTemplateBinding {
    pub fn new(
        annotations: Option<std::collections::HashMap<String, String>>,
        group_name: Option<String>,
        group_principal_name: Option<String>,
        id: String,
        labels: Option<std::collections::HashMap<String, String>>,
        namespace: String,
        project_name: String,
        uid: Option<String>,
        role_template_name: String,
        resource_version: Option<String>,
        service_account: Option<String>,
        user_name: Option<String>,
        user_principal_name: Option<String>,
    ) -> Self {
        ProjectRoleTemplateBinding {
            annotations,
            group_name,
            group_principal_name,
            id,
            labels,
            namespace,
            project_name,
            role_template_name,
            resource_version,
            service_account,
            uid,
            user_name,
            user_principal_name,
        }
    }
}

impl TryFrom<IoCattleManagementv3ProjectRoleTemplateBinding> for ProjectRoleTemplateBinding {
    type Error = anyhow::Error;

    fn try_from(
        value: IoCattleManagementv3ProjectRoleTemplateBinding,
    ) -> Result<Self, Self::Error> {
        let metadata: IoK8sApimachineryPkgApisMetaV1ObjectMeta = value.metadata.ok_or_else(|| anyhow::anyhow!("Missing metadata field"))?;

        let id = metadata.name.ok_or_else(|| anyhow::anyhow!("Missing metadata.name"))?;

        // Extract the fields from the IoCattleManagementv3ProjectRoleTemplateBinding
        // and create a new ProjectRoleTemplateBinding instance
        let group_name = value.group_name;
        let group_principal_name = value.group_principal_name;
        let project_name = value.project_name;
        let role_template_name = value.role_template_name;
        let service_account = value.service_account;
        let user_name = value.user_name;
        let user_principal_name = value.user_principal_name;
        let annotations = metadata.annotations.map(|a| {
            a.into_iter()
                .collect::<std::collections::HashMap<String, String>>()
        });

        let labels = metadata.labels.map(|a| {
            a.into_iter()
                .collect::<std::collections::HashMap<String, String>>()
        });
        let namespace = metadata.namespace.unwrap_or_default();
        let resource_version = metadata.resource_version;
        let uid = metadata.uid;

        Ok(ProjectRoleTemplateBinding {
            id,
            group_name,
            group_principal_name,
            project_name,
            role_template_name,
            service_account,
            user_name,
            user_principal_name,
            annotations,
            labels,
            namespace,
            resource_version,
            uid,
        })
    }
}

impl TryFrom<ProjectRoleTemplateBinding> for IoCattleManagementv3ProjectRoleTemplateBinding {
    type Error = anyhow::Error;

    fn try_from(value: ProjectRoleTemplateBinding) -> Result<Self, Self::Error> {
        // Create a new IoCattleManagementv3ProjectRoleTemplateBinding instance
        let metadata = IoK8sApimachineryPkgApisMetaV1ObjectMeta {
            annotations: value.annotations,
            labels: value.labels,
            namespace: Some(value.namespace),
            name: Some(value.id.clone()),
            ..Default::default()
        };

        Ok(IoCattleManagementv3ProjectRoleTemplateBinding {
            api_version: Some("management.cattle.io/v3".to_string()),
            group_name: value.group_name,
            group_principal_name: value.group_principal_name,
            kind: Some("ProjectRoleTemplateBinding".to_string()),
            metadata: Some(metadata),
            project_name: value.project_name,
            role_template_name: value.role_template_name,
            service_account: value.service_account,
            user_name: value.user_name,
            user_principal_name: value.user_principal_name,
        })
    }
}

impl PartialEq<ProjectRoleTemplateBinding> for IoCattleManagementv3ProjectRoleTemplateBinding {
    fn eq(&self, other: &ProjectRoleTemplateBinding) -> bool {
        let lhs = self.metadata.as_ref().and_then(|m| m.name.clone());
        let rhs = Some(other.id.clone());

        lhs == rhs
            && self.group_name == other.group_name
            && self.group_principal_name == other.group_principal_name
            && self.project_name == other.project_name
            && self.role_template_name == other.role_template_name
            && self.service_account == other.service_account
            && self.user_name == other.user_name
            && self.user_principal_name == other.user_principal_name
    }
}

impl PartialEq<IoCattleManagementv3ProjectRoleTemplateBinding> for ProjectRoleTemplateBinding {
    fn eq(&self, other: &IoCattleManagementv3ProjectRoleTemplateBinding) -> bool {
        // let lhs = Some(self.id.clone());
        // let rhs = other.metadata.as_ref().and_then(|m| m.name.clone());

        // lhs == rhs
        //     && self.group_name == other.group_name
        //     && self.group_principal_name == other.group_principal_name
        //     && self.project_name == other.project_name
        //     && self.role_template_name == other.role_template_name
        //     && self.service_account == other.service_account
        //     && self.user_name == other.user_name
        //     && self.user_principal_name == other.user_principal_name

        other == self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_binding() -> ProjectRoleTemplateBinding {
        ProjectRoleTemplateBinding {
            id: "binding-id".to_string(),
            group_name: Some("group1".to_string()),
            group_principal_name: Some("groupPrincipal".to_string()),
            project_name: "project-id".to_string(),
            role_template_name: "role-template".to_string(),
            service_account: Some("service-account".to_string()),
            user_name: Some("user1".to_string()),
            user_principal_name: Some("userPrincipal".to_string()),
            annotations: Some(std::collections::HashMap::new()),
            labels: Some(std::collections::HashMap::new()),
            namespace: "namespace-id".to_string(),
            resource_version: Some("resource-version".to_string()),
            uid: Some("uid".to_string()),
        }
    }

    fn sample_iocattle_binding() -> IoCattleManagementv3ProjectRoleTemplateBinding {
        IoCattleManagementv3ProjectRoleTemplateBinding {
            api_version: Some("management.cattle.io/v3".to_string()),
            kind: Some("ProjectRoleTemplateBinding".to_string()),
            metadata: Some(IoK8sApimachineryPkgApisMetaV1ObjectMeta {
                name: Some("binding-id".to_string()),
                ..Default::default()
            }),
            group_name: Some("group1".to_string()),
            group_principal_name: Some("groupPrincipal".to_string()),
            project_name: "project-id".to_string(),
            role_template_name: "role-template".to_string(),
            service_account: Some("service-account".to_string()),
            user_name: Some("user1".to_string()),
            user_principal_name: Some("userPrincipal".to_string()),
        }
    }

    #[test]
    fn test_equality_both_directions() {
        let a = sample_binding();
        let b = sample_iocattle_binding();

        assert_eq!(a, b);
        assert_eq!(b, a);
    }

    #[test]
    fn test_try_from_iocattle_to_binding() {
        let ioc = sample_iocattle_binding();
        let result = ProjectRoleTemplateBinding::try_from(ioc);
        assert!(result.is_ok());

        let binding = result.unwrap();
        assert_eq!(binding.id, "binding-id");
        assert_eq!(binding.group_name.as_deref(), Some("group1"));
    }

    #[test]
    fn test_try_from_binding_to_iocattle() {
        let binding = sample_binding();
        let result = IoCattleManagementv3ProjectRoleTemplateBinding::try_from(binding);
        assert!(result.is_ok());

        let ioc = result.unwrap();
        assert_eq!(ioc.metadata.unwrap().name, Some("binding-id".to_string()));
        assert_eq!(ioc.group_name.as_deref(), Some("group1"));
    }

    #[test]
    fn test_inequality_on_different_user() {
        let a = sample_binding();
        let mut b = sample_iocattle_binding();

        b.user_name = Some("other-user".to_string());

        assert_ne!(a, b);
        assert_ne!(b, a);
    }

    #[test]
    fn test_missing_metadata_name() {
        let mut b = sample_iocattle_binding();
        b.metadata.as_mut().unwrap().name = None;

        let result = ProjectRoleTemplateBinding::try_from(b);
        assert!(result.is_err());
    }

    #[test]
    fn test_inequality_on_missing_metadata_name_in_eq() {
        let a = sample_binding();
        let mut b = sample_iocattle_binding();
        b.metadata.as_mut().unwrap().name = None;

        assert_ne!(a, b);
        assert_ne!(b, a);
    }
}
