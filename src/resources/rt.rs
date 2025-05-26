use crate::utils::logging::log_api_error;
use anyhow::{Context as anyhow_context, Result};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use rancher_client::{apis::{configuration::Configuration, management_cattle_io_v3_api::{create_management_cattle_io_v3_role_template, read_management_cattle_io_v3_role_template}}, models::{IoK8sApimachineryPkgApisMetaV1Patch, IoK8sApimachineryPkgApisMetaV1Status}};
use reqwest::StatusCode;

use rancher_client::{
    apis::management_cattle_io_v3_api::{
        patch_management_cattle_io_v3_role_template,
        list_management_cattle_io_v3_role_template,
        delete_management_cattle_io_v3_role_template
    },
    models::io_cattle_managementv3_role_template::Context,
    models::{
        IoCattleManagementv3GlobalRoleRulesInner, IoCattleManagementv3RoleTemplate,
        IoCattleManagementv3RoleTemplateList, IoK8sApimachineryPkgApisMetaV1ObjectMeta,
    },
};
use serde_json::Value;
use tracing::{debug, info, trace};


pub const RT_EXCLUDE_PATHS: &[&str] = &[
    "metadata.creationTimestamp",
    "metadata.finalizers",
    "metadata.generateName",
    "metadata.generation",
    "metadata.managedFields",
    "metadata.resourceVersion",
    "metadata.selfLink",
    "metadata.uid",
];

/// Find a role template by its ID
///
/// # Arguments
///
/// * `configuration` - The configuration to use for the request
/// * `role_template_id` - The ID of the role template to get
/// * `resource_version` - The resource version to use for the request
/// # Returns
///
/// * `IoCattleManagementv3RoleTemplate` - The role template
/// # Errors
///
/// * `anyhow::Error` - The error that occurred while trying to get the role template
///
#[async_backtrace::framed]
pub async fn find_role_template(
    configuration: &Configuration,
    role_template_id: &str,
    resource_version: Option<&str>,
) -> Result<IoCattleManagementv3RoleTemplate> {
    debug!("Reading role template with ID: {}", role_template_id);

    let api_result = read_management_cattle_io_v3_role_template(
        configuration,
        role_template_id,
        None,
        resource_version,
    )
    .await
    .context(format!(
        "Failed to find role template with ID: {}",
        role_template_id
    ));

    match api_result {
        Err(e) => {
            log_api_error("find_role_template", &e);
            Err(anyhow::anyhow!(e))
        }
        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");

            match response_content.status {
                StatusCode::OK => {
                    match serde_json::from_str::<IoCattleManagementv3RoleTemplate>(&response_content.content) {
                        Ok(data) => {
                            info!("Successfully found role template with ID: {}", role_template_id);
                            Ok(data)
                        }
                        Err(deserialize_err) => {
                            let err = anyhow::anyhow!(
                                "Failed to deserialize role template response: {}",
                                deserialize_err
                            );
                            log_api_error("find_role_template:deserialize", &err);
                            Err(err)
                        }
                    }
                }
                StatusCode::NOT_FOUND => {
                    let err = anyhow::anyhow!(
                        "Role template with ID: {} not found. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("find_role_template:not_found", &err);
                    Err(err)
                }
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized access while trying to find role template with ID: {}. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("find_role_template:unauthorized", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden access while trying to find role template with ID: {}. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("find_role_template:forbidden", &err);
                    Err(err)
                }
                status => {
                    let err = match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when finding role template with ID: {}: {}",
                                status, role_template_id, serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when finding role template with ID: {}: {}",
                                status, role_template_id, response_content.content
                            )
                        }
                    };
                    log_api_error("find_role_template:unexpected_status", &err);
                    Err(err)
                }
            }
        }
    }
}





/// Delete a role template by its ID
/// # Arguments
/// * `configuration` - The configuration to use for the request
/// * `role_template_id` - The ID of the role template to delete
/// # Returns
/// * `IoK8sApimachineryPkgApisMetaV1Status` - The status of the deletion
/// # Errors
/// * `anyhow::Error` - The error that occurred while trying to delete the role template
///
#[async_backtrace::framed]
pub async fn delete_role_template(
    configuration: &Configuration,
    role_template_id: &str,
) -> Result<IoK8sApimachineryPkgApisMetaV1Status> {
    info!("Deleting role template with ID: {}", role_template_id);

    let result = delete_management_cattle_io_v3_role_template(
        configuration,
        role_template_id,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .context(format!(
        "Failed to delete role template with ID: {}",
        role_template_id
    ));

    match result {
        Err(e) => {
            log_api_error("delete_role_template", &e);
            Err(anyhow::anyhow!(e))
        }


        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");

            match response_content.status {
                StatusCode::OK => {
                    match serde_json::from_str::<IoK8sApimachineryPkgApisMetaV1Status>(&response_content.content) {
                        Ok(data) => {
                            info!("Successfully deleted role template with ID: {}", role_template_id);
                            Ok(data)
                        }
                        Err(deserialize_err) => {
                            let err = anyhow::anyhow!(
                                "Failed to deserialize role template deletion response: {}",
                                deserialize_err
                            );
                            log_api_error("delete_role_template:deserialize", &err);
                            Err(err)
                        }
                    }
                }
                StatusCode::NOT_FOUND => {
                    let err = anyhow::anyhow!(
                        "Role template with ID: {} not found for deletion. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("delete_role_template:not_found", &err);
                    Err(err)
                }
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized to delete role template with ID: {}. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("delete_role_template:unauthorized", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden to delete role template with ID: {}. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("delete_role_template:forbidden", &err);
                    Err(err)
                }
                StatusCode::CONFLICT => {
                    let err = anyhow::anyhow!(
                        "Conflict when deleting role template with ID: {}. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("delete_role_template:conflict", &err);
                    Err(err)
                }
                status => {
                    let err = match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when deleting role template with ID: {}: {}", 
                                status, 
                                role_template_id,
                                serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when deleting role template with ID: {}: {}", 
                                status, 
                                role_template_id,
                                response_content.content
                            )
                        }
                    };
                    log_api_error("delete_role_template:unexpected_status", &err);
                    Err(err)
                }
            }
        }
    }
}


/// Get all role templates
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
/// * `IoCattleManagementv3RoleTemplateList` - The list of role templates
/// # Errors
///
/// * `anyhow::Error` - The error that occurred while trying to get the role templates
///
#[async_backtrace::framed]
pub async fn get_role_templates(
    configuration: &Configuration,
    field_selector: Option<&str>,
    label_selector: Option<&str>,
    limit: Option<i32>,
    resource_version: Option<&str>,
    resource_version_match: Option<&str>,
    continue_: Option<&str>,
) -> Result<IoCattleManagementv3RoleTemplateList> {
    let api_result = list_management_cattle_io_v3_role_template(
        configuration,
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
    .context("Failed to get role templates");

    match api_result {
        Err(e) => {
            log_api_error("get_role_templates", &e);
            Err(anyhow::anyhow!(e))
        }
        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");
            
            match response_content.status {
                StatusCode::OK => {
                    match serde_json::from_str::<IoCattleManagementv3RoleTemplateList>(&response_content.content) {
                        Ok(data) => {
                            debug!("Successfully retrieved {} role templates", data.items.len());
                            Ok(data)
                        },
                        Err(deserialize_err) => {
                            let err = anyhow::anyhow!("Failed to deserialize role templates response: {}", deserialize_err);
                            log_api_error("get_role_templates:deserialize", &err);
                            Err(err)
                        }
                    }
                },
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized to get role templates. Please check your credentials. Response: {}",
                        response_content.content
                    );
                    log_api_error("get_role_templates:unauthorized", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden to get role templates. Please check your roles. Response: {}",
                        response_content.content
                    );
                    log_api_error("get_role_templates:forbidden", &err);
                    Err(err)
                }
                status => {
                    // For other status codes, try to parse error details from response
                    let err = match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when getting role templates: {}", 
                                status, 
                                serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when getting role templates: {}", 
                                status, 
                                response_content.content
                            )
                        }
                    };
                    log_api_error("get_role_templates:unexpected_status", &err);
                    Err(err)
                }
            }
        }
    }
}




/// Update a role template by its ID
/// # Arguments
/// * `configuration` - The configuration to use for the request
/// * `role_template_id` - The ID of the role template to update
/// * `patch_value` - The JSON patch to apply
/// # Returns
/// * `IoCattleManagementv3RoleTemplate` - The updated role template
/// # Errors
/// * `anyhow::Error` - The error that occurred while trying to update the role template
///
#[async_backtrace::framed]
pub async fn update_role_template(
    configuration: &Configuration,
    role_template_id: &str,
    patch_value: Value,
) -> Result<IoCattleManagementv3RoleTemplate> {
    info!("Patching role template with ID: {}", role_template_id);

    let patch_array = match patch_value {
        Value::Array(arr) => arr,
        Value::Null => {
            let err = anyhow::anyhow!("Expected patch to serialize to a JSON array, but got null");
            log_api_error("update_role_template:invalid_patch", &err);
            return Err(err);
        }
        _ => {
            let err = anyhow::anyhow!(
                "Expected patch to serialize to a JSON array, but got: {:?}",
                patch_value
            );
            log_api_error("update_role_template:invalid_patch", &err);
            return Err(err);
        }
    };

    let k8s_patch = IoK8sApimachineryPkgApisMetaV1Patch::Array(patch_array);

    let result = patch_management_cattle_io_v3_role_template(
        configuration,
        role_template_id,
        Some(k8s_patch),
        None,
        None,
        None,
        None,
        None
    )
    .await
    .context(format!(
        "Failed to update role template with ID: {}",
        role_template_id
    ));

    match result {
        Err(e) => {
            log_api_error("update_role_template", &e);
            Err(anyhow::anyhow!(e))
        }
        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");

            match response_content.status {
                StatusCode::OK => {
                    match serde_json::from_str::<IoCattleManagementv3RoleTemplate>(&response_content.content) {
                        Ok(data) => {
                            info!("Successfully updated role template with ID: {}", role_template_id);
                            Ok(data)
                        }
                        Err(deserialize_err) => {
                            let err = anyhow::anyhow!(
                                "Failed to deserialize role template update response: {}",
                                deserialize_err
                            );
                            log_api_error("update_role_template:deserialize", &err);
                            Err(err)
                        }
                    }
                }
                StatusCode::NOT_FOUND => {
                    let err = anyhow::anyhow!(
                        "Role template with ID: {} not found for update. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("update_role_template:not_found", &err);
                    Err(err)
                }
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized to update role template with ID: {}. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("update_role_template:unauthorized", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden to update role template with ID: {}. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("update_role_template:forbidden", &err);
                    Err(err)
                }
                StatusCode::CONFLICT => {
                    let err = anyhow::anyhow!(
                        "Conflict when updating role template with ID: {}. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("update_role_template:conflict", &err);
                    Err(err)
                }
                status => {
                    let err = match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when updating role template with ID: {}: {}", 
                                status, 
                                role_template_id,
                                serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when updating role template with ID: {}: {}", 
                                status, 
                                role_template_id,
                                response_content.content
                            )
                        }
                    };
                    log_api_error("update_role_template:unexpected_status", &err);
                    Err(err)
                }
            }
        }
    }
}



/// Create a role template
///
/// # Arguments
///
/// * `configuration` - The configuration to use for the request
/// * `body` - The role template to create
/// # Returns
///
/// * `IoCattleManagementv3RoleTemplate` - The created role template
/// # Errors
///
/// * `anyhow::Error` - The error that occurred while trying to create the role template
///
#[async_backtrace::framed]
pub async fn create_role_template(
    configuration: &Configuration,
    body: IoCattleManagementv3RoleTemplate,
) -> Result<IoCattleManagementv3RoleTemplate> {
    let role_template_id = body.metadata.as_ref().unwrap().name.clone().unwrap_or_default();
    info!("Creating role template with ID: {}", role_template_id);

    let result = create_management_cattle_io_v3_role_template(
        configuration,
        body,
        None,
        None,
        None,
        None,
    )
    .await
    .context(format!(
        "Failed to create role template with ID: {}",
        role_template_id
    ));

    match result {
        Err(e) => {
            log_api_error("create_role_template", &e);
            Err(anyhow::anyhow!(e))
        }
        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");

            match response_content.status {
                StatusCode::CREATED | StatusCode::OK => {
                    match serde_json::from_str::<IoCattleManagementv3RoleTemplate>(&response_content.content) {
                        Ok(data) => {
                            info!("Successfully created role template with ID: {}", role_template_id);
                            Ok(data)
                        }
                        Err(deserialize_err) => {
                            let err = anyhow::anyhow!(
                                "Failed to deserialize role template creation response: {}",
                                deserialize_err
                            );
                            log_api_error("create_role_template:deserialize", &err);
                            Err(err)
                        }
                    }
                }
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized to create role template with ID: {}. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("create_role_template:unauthorized", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden to create role template with ID: {}. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("create_role_template:forbidden", &err);
                    Err(err)
                }
                StatusCode::CONFLICT => {
                    let err = anyhow::anyhow!(
                        "Conflict when creating role template with ID: {}. Response: {}",
                        role_template_id,
                        response_content.content
                    );
                    log_api_error("create_role_template:conflict", &err);
                    Err(err)
                }
                status => {
                    let err = match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when creating role template with ID: {}: {}", 
                                status, 
                                role_template_id,
                                serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when creating role template with ID: {}: {}", 
                                status, 
                                role_template_id,
                                response_content.content
                            )
                        }
                    };
                    log_api_error("create_role_template:unexpected_status", &err);
                    Err(err)
                }
            }
        }
    }
}



#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RoleTemplate {

    /// Administrative if true, this RoleTemplate is used to grant administrative privileges. Default to false.
    /// This field is not set in the API, but is used to determine if the role template is administrative
    #[serde(skip_serializing_if = "Option::is_none")]
    pub administrative: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builtin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_creator_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Context>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_creator_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_template_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<IoCattleManagementv3GlobalRoleRulesInner>>,
}

impl RoleTemplate {
    pub fn new(
        administrative: Option<bool>,
        annotations: Option<HashMap<String, String>>,
        builtin: Option<bool>,
        cluster_creator_default: Option<bool>,
        context: Option<Context>,
        description: Option<String>,
        display_name: Option<String>,
        external: Option<bool>,
        hidden: Option<bool>,
        labels: Option<HashMap<String, String>>,
        locked: Option<bool>,
        id: String,
        project_creator_default: Option<bool>,
        resource_version: Option<String>,
        role_template_names: Option<Vec<String>>,
        rules: Option<Vec<IoCattleManagementv3GlobalRoleRulesInner>>,
    ) -> Self {
        RoleTemplate {
            annotations,
            administrative,
            builtin,
            cluster_creator_default,
            context,
            description,
            display_name,
            external,
            hidden,
            labels,
            locked,
            id,
            resource_version,
            project_creator_default,
            role_template_names,
            rules,
        }
    }
}

impl TryFrom<IoCattleManagementv3RoleTemplate> for RoleTemplate {
    type Error = anyhow::Error;

    fn try_from(value: IoCattleManagementv3RoleTemplate) -> Result<Self, Self::Error> {
        let metadata: IoK8sApimachineryPkgApisMetaV1ObjectMeta = value.metadata.ok_or_else(|| anyhow::anyhow!("Missing metadata"))?;

        let administrative: Option<bool> = value.administrative;
        let annotations: Option<HashMap<String, String>> = metadata.annotations;
        let builtin: Option<bool> = value.builtin;
        let cluster_creator_default: Option<bool> = value.cluster_creator_default;
        let context = value.context;
        let description: Option<String> = value.description;
        let display_name: Option<String> = value.display_name;
        let external: Option<bool> = value.external;
        let hidden: Option<bool> = value.hidden;
        let labels: Option<HashMap<String, String>> = metadata.labels;
        let locked: Option<bool> = value.locked;
        let project_creator_default: Option<bool> = value.project_creator_default;
        let resource_version: Option<String> = metadata.resource_version;
        let role_template_names: Option<Vec<String>> = value.role_template_names;
        let rules: Option<Vec<IoCattleManagementv3GlobalRoleRulesInner>> = value.rules;

        Ok(RoleTemplate {
            administrative,
            annotations,
            builtin,
            cluster_creator_default,
            context,
            description,
            display_name,
            external,
            hidden,
            id: metadata.name.ok_or_else(|| anyhow::anyhow!("Missing metadata.name"))?,
            labels,
            locked,
            project_creator_default,
            resource_version,
            role_template_names,
            rules,
        })
    }
}

impl TryFrom<RoleTemplate> for IoCattleManagementv3RoleTemplate {
    type Error = anyhow::Error;

    fn try_from(value: RoleTemplate) -> Result<Self, Self::Error> {
        let metadata = IoK8sApimachineryPkgApisMetaV1ObjectMeta {
            annotations: value.annotations,
            labels: value.labels,
            name: Some(value.id.clone()),
            ..Default::default()
        };

        let context = value.context;
        let administrative: Option<bool> = value.administrative;
        let builtin: Option<bool> = value.builtin;
        let cluster_creator_default: Option<bool> = value.cluster_creator_default;
        let description: Option<String> = value.description;
        let display_name: Option<String> = value.display_name;
        let external: Option<bool> = value.external;
        let hidden: Option<bool> = value.hidden;
        let locked: Option<bool> = value.locked;
        let project_creator_default: Option<bool> = value.project_creator_default;
        let role_template_names: Option<Vec<String>> = value.role_template_names;
        let rules: Option<Vec<IoCattleManagementv3GlobalRoleRulesInner>> = value.rules;

        Ok(IoCattleManagementv3RoleTemplate {
            administrative,
            api_version: Some("management.cattle.io/v3".to_string()),
            builtin,
            cluster_creator_default,
            context,
            description,
            display_name,
            external,
            hidden,
            kind: Some("RoleTemplate".to_string()),
            locked,
            metadata: Some(metadata),
            project_creator_default,
            role_template_names,
            rules,
        })
    }
}

impl PartialEq<RoleTemplate> for IoCattleManagementv3RoleTemplate {
    fn eq(&self, other: &RoleTemplate) -> bool {
        let lhs = self.metadata.as_ref().and_then(|m| m.name.clone());
        let rhs = Some(other.id.clone());

        lhs == rhs
            && self.administrative == other.administrative
            && self.builtin == other.builtin
            && self.cluster_creator_default == other.cluster_creator_default
            && self.context == other.context
            && self.description == other.description
            && self.display_name == other.display_name
            && self.external == other.external
            && self.hidden == other.hidden
            && self.locked == other.locked
            && self.project_creator_default == other.project_creator_default
            && self.role_template_names == other.role_template_names
            && self.rules == other.rules
    }
}


impl PartialEq<IoCattleManagementv3RoleTemplate> for RoleTemplate {
    fn eq(&self, other: &IoCattleManagementv3RoleTemplate) -> bool {
        // let lhs = Some(self.id.clone());
        // let rhs = other.metadata.as_ref().and_then(|m| m.name.clone());

        // self.administrative == other.administrative
        //     && self.builtin == other.builtin
        //     && self.cluster_creator_default == other.cluster_creator_default
        //     && self.context == other.context
        //     && self.description == other.description
        //     && self.display_name == other.display_name
        //     && self.external == other.external
        //     && self.hidden == other.hidden
        //     && self.locked == other.locked
        //     && lhs == rhs
        //     && self.project_creator_default == other.project_creator_default
        //     && self.role_template_names == other.role_template_names
        //     && self.rules == other.rules


        other == self
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;

    fn sample_metadata(name: &str) -> IoK8sApimachineryPkgApisMetaV1ObjectMeta {
        IoK8sApimachineryPkgApisMetaV1ObjectMeta {
            name: Some(name.to_string()),
            ..Default::default()
        }
    }

    fn sample_role_template() -> RoleTemplate {
        RoleTemplate::new(
            Some(true), // administrative
            Some(std::collections::HashMap::new()), // annotations
            Some(false), // builtin
            Some(true), // cluster_creator_default
            Some(Context::Cluster), // context
            Some("A role template".to_string()), // description
            Some("Admin".to_string()), // display_name
            Some(false), // external
            Some(false), // hidden
            Some(std::collections::HashMap::new()), // labels
            Some(false), // locked
            "admin-template".to_string(), // id
            Some(false), // project_creator_default
            None,
            Some(vec!["base-template".to_string()]), // role_template_names
            Some(vec![]), // rules
        )
    }

    fn sample_iocattle_role_template() -> IoCattleManagementv3RoleTemplate {
        IoCattleManagementv3RoleTemplate {
            administrative: Some(true),
            api_version: Some("management.cattle.io/v3".to_string()),
            builtin: Some(false),
            cluster_creator_default: Some(true),
            context: Some(Context::Cluster),
            description: Some("A role template".to_string()),
            display_name: Some("Admin".to_string()),
            external: Some(false),
            hidden: Some(false),
            kind: Some("RoleTemplate".to_string()),
            locked: Some(false),
            metadata: Some(sample_metadata("admin-template")),
            project_creator_default: Some(false),
            role_template_names: Some(vec!["base-template".to_string()]),
            rules: Some(vec![]),
        }
    }

    #[test]
    fn test_iocattle_to_role_template_conversion_success() {
        let io_rt = sample_iocattle_role_template();
        let result = RoleTemplate::try_from(io_rt).unwrap();
        assert_eq!(result.id, "admin-template");
        assert_eq!(result.display_name.as_deref(), Some("Admin"));
        assert_eq!(result.project_creator_default, Some(false));
    }

    #[test]
    fn test_role_template_to_iocattle_conversion_success() {
        let rt = sample_role_template();
        let result = IoCattleManagementv3RoleTemplate::try_from(rt).unwrap();
        assert_eq!(
            result.metadata.as_ref().unwrap().name.as_deref(),
            Some("admin-template")
        );
        assert_eq!(result.display_name.as_deref(), Some("Admin"));
        assert_eq!(result.project_creator_default, Some(false));
    }

    #[test]
    fn test_iocattle_to_role_template_missing_metadata() {
        let mut io_rt = sample_iocattle_role_template();
        io_rt.metadata = None;
        let result = RoleTemplate::try_from(io_rt);
        assert!(result.is_err());
    }

    #[test]
    fn test_iocattle_to_role_template_missing_metadata_name() {
        let mut io_rt = sample_iocattle_role_template();
        io_rt.metadata.as_mut().unwrap().name = None;
        let result = RoleTemplate::try_from(io_rt);
        assert!(result.is_err());
    }

    #[test]
    fn test_round_trip_conversion() {
        let original = sample_role_template();
        let back_and_forth = RoleTemplate::try_from(
            IoCattleManagementv3RoleTemplate::try_from(original.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(original, back_and_forth);
    }


    #[test]
    fn test_equality_both_directions() {
        let rt = sample_role_template();
        let iort = sample_iocattle_role_template();

        // Forward comparison
        assert_eq!(iort, rt);

        // Reverse comparison
        assert_eq!(rt, iort);
    }

    #[test]
    fn test_inequality_on_field() {
        let rt = sample_role_template();
        let mut iort = sample_iocattle_role_template();

        // Change display name
        iort.display_name = Some("Changed".into());

        assert_ne!(rt, iort);
        assert_ne!(iort, rt);
    }

    #[test]
    fn test_missing_metadata_name() {
        let rt = sample_role_template();
        let mut iort = sample_iocattle_role_template();

        // Remove the name field
        if let Some(metadata) = iort.metadata.as_mut() {
            metadata.name = None;
        }

        assert_ne!(rt, iort);
        assert_ne!(iort, rt);
    }



}
