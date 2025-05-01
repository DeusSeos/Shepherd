use serde::{Deserialize, Serialize};

use rancher_client::apis::{configuration::Configuration, Error, ResponseContent};
use reqwest::StatusCode;

use rancher_client::{
    apis::management_cattle_io_v3_api::{
      list_management_cattle_io_v3_namespaced_project_role_template_binding, ListManagementCattleIoV3NamespacedProjectRoleTemplateBindingError,
      list_management_cattle_io_v3_project_role_template_binding_for_all_namespaces, ListManagementCattleIoV3ProjectRoleTemplateBindingForAllNamespacesError,
    },
    models::{

        IoCattleManagementv3ProjectRoleTemplateBinding, IoCattleManagementv3ProjectRoleTemplateBindingList,
         IoK8sApimachineryPkgApisMetaV1ObjectMeta,
    },
    // models::io_cattle_managementv3_role_template::Context,
};

/// Get all project role template bindings from an endpoint using the provided configuration
///
/// # Arguments
///
/// * `configuration` - The configuration to use for the request
///
/// # Returns
///
/// * `IoCattleManagementv3ProjectRoleTemplateBindingList` - The list of project role template bindings
///
/// # Errors
///
/// * `Error<ListManagementCattleIoV3ProjectRoleTemplateBindingForAllNamespacesError>` - The error that occurred while trying to get the bindings
///
pub async fn get_project_role_template_bindings(
    configuration: &Configuration,
) -> Result<IoCattleManagementv3ProjectRoleTemplateBindingList, Error<ListManagementCattleIoV3ProjectRoleTemplateBindingForAllNamespacesError>> {
    let result = list_management_cattle_io_v3_project_role_template_binding_for_all_namespaces(
        configuration,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await;
    match result {
        Err(e) => Err(e),
        Ok(response_content) => {
            // Match on the status code and deserialize accordingly
            match response_content.status {
                StatusCode::OK => {
                    // Try to deserialize the content into IoCattleManagementv3ProjectRoleTemplateBindingList (Status200 case)
                    match serde_json::from_str(&response_content.content) {
                        Ok(data) => Ok(data),
                        Err(deserialize_err) => Err(Error::Serde(deserialize_err)),
                    }
                }
                _ => {
                    // If not status 200, treat as UnknownValue
                    match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(unknown_data) => {
                            // Handle the unknown response
                            Err(Error::ResponseError(ResponseContent {
                                status: response_content.status,
                                content: response_content.content,
                                entity: Some(ListManagementCattleIoV3ProjectRoleTemplateBindingForAllNamespacesError::UnknownValue(
                                    unknown_data,
                                )),
                            }))
                        }
                        Err(deserialize_err) => Err(Error::Serde(deserialize_err)),
                    }
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
/// * `cluster_id` - The ID of the cluster (namespace) to get the project role template bindings for
///
/// # Returns
///
/// * `IoCattleManagementv3ProjectRoleTemplateBindingList` - The list of project role template bindings
///
/// # Errors
///
/// * `Error<ListManagementCattleIoV3ProjectRoleTemplateBindingForAllNamespacesError>` - The error that occurred while trying to get the bindings
pub async fn get_namespaced_project_role_template_bindings(
    configuration: &Configuration,
    project_id: &str,
) -> Result<IoCattleManagementv3ProjectRoleTemplateBindingList, Error<ListManagementCattleIoV3NamespacedProjectRoleTemplateBindingError>> {
    let result = list_management_cattle_io_v3_namespaced_project_role_template_binding(
        configuration,
        project_id,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await;
    match result {
        Err(e) => Err(e),
        Ok(response_content) => {
            // Match on the status code and deserialize accordingly
            match response_content.status {
                StatusCode::OK => {
                    // Try to deserialize the content into IoCattleManagementv3ProjectRoleTemplateBindingList (Status200 case)
                    match serde_json::from_str(&response_content.content) {
                        Ok(data) => Ok(data),
                        Err(deserialize_err) => Err(Error::Serde(deserialize_err)),
                    }
                }
                _ => {
                    // If not status 200, treat as UnknownValue
                    match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(unknown_data) => {
                            // Handle the unknown response
                            Err(Error::ResponseError(ResponseContent {
                                status: response_content.status,
                                content: response_content.content,
                                entity: Some(ListManagementCattleIoV3NamespacedProjectRoleTemplateBindingError::UnknownValue(
                                    unknown_data,
                                )),
                            }))
                        }
                        Err(deserialize_err) => Err(Error::Serde(deserialize_err)),
                    }
                }
            }
        }
    }
}




#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectRoleTemplateBinding {
    /// The name of the project role template binding (typically the Kubernetes metadata.name).
    pub id: String,
    pub group_name: String,
    pub group_principal_name: String,
    pub project_name: String,
    pub role_template_name: String,
    pub service_account: String,
    pub user_name: String,
    pub user_principal_name: String,
}

impl ProjectRoleTemplateBinding {
    pub fn new(
        id: String,
        group_name: String,
        group_principal_name: String,
        project_name: String,
        role_template_name: String,
        service_account: String,
        user_name: String,
        user_principal_name: String) -> Self {
        ProjectRoleTemplateBinding {
            group_name,
            group_principal_name,
            id,
            project_name,
            role_template_name,
            service_account,
            user_name,
            user_principal_name
        }
    }
}

impl TryFrom<IoCattleManagementv3ProjectRoleTemplateBinding> for ProjectRoleTemplateBinding {
    type Error = &'static str;

    fn try_from(value: IoCattleManagementv3ProjectRoleTemplateBinding) -> Result<Self, Self::Error> {
        let metadata: IoK8sApimachineryPkgApisMetaV1ObjectMeta =
            *value.metadata.ok_or("missing metadata")?;
        
        Ok(ProjectRoleTemplateBinding {
            group_name: value.group_name.unwrap_or_else(|| "".to_string()),
            group_principal_name: value.group_principal_name.unwrap_or_else(|| "".to_string()),
            id: metadata.name.ok_or("missing metadata.name")?,
            project_name: value.project_name,
            role_template_name: value.role_template_name,
            service_account: value.service_account.unwrap_or_else(|| "".to_string()),
            user_name: value.user_name.unwrap_or_else(|| "".to_string()),
            user_principal_name: value.user_principal_name.unwrap_or_else(|| "".to_string()),
        })
    }
}

