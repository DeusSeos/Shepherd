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




#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProjectRoleTemplateBinding {
    /// The name of the project role template binding (typically the Kubernetes metadata.name).
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_principal_name: Option<String>,
    pub project_name: String,
    pub role_template_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_principal_name: Option<String>,
}

impl ProjectRoleTemplateBinding {
    pub fn new(
        id: String,
        group_name: Option<String>,
        group_principal_name: Option<String>,
        project_name: String,
        role_template_name: String,
        service_account: Option<String>,
        user_name: Option<String>,
        user_principal_name: Option<String>
    ) -> Self {
        ProjectRoleTemplateBinding {
            id,
            group_name,
            group_principal_name,
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

        let id = metadata.name.ok_or("missing name")?;

        // Extract the fields from the IoCattleManagementv3ProjectRoleTemplateBinding
        // and create a new ProjectRoleTemplateBinding instance
        let group_name = value.group_name;
        let group_principal_name = value.group_principal_name;
        let project_name = value.project_name;
        let role_template_name = value.role_template_name;
        let service_account = value.service_account;
        let user_name = value.user_name;
        let user_principal_name = value.user_principal_name;

        
        Ok(ProjectRoleTemplateBinding {
            id,
            group_name,
            group_principal_name,
            project_name,
            role_template_name,
            service_account,
            user_name,
            user_principal_name
            
        })
    }
}


impl TryFrom<ProjectRoleTemplateBinding> for IoCattleManagementv3ProjectRoleTemplateBinding {
    type Error = &'static str;

    fn try_from(value: ProjectRoleTemplateBinding) -> Result<Self, Self::Error> {
        // Create a new IoCattleManagementv3ProjectRoleTemplateBinding instance
        let metadata = IoK8sApimachineryPkgApisMetaV1ObjectMeta {
            name: Some(value.id.clone()),
            ..Default::default()
        };

        Ok(IoCattleManagementv3ProjectRoleTemplateBinding {
            api_version: Some("management.cattle.io/v3".to_string()),
            group_name: value.group_name,
            group_principal_name: value.group_principal_name,
            kind: Some("ProjectRoleTemplateBinding".to_string()),
            metadata: Some(Box::new(metadata)),
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
        }
    }

    fn sample_iocattle_binding() -> IoCattleManagementv3ProjectRoleTemplateBinding {
        IoCattleManagementv3ProjectRoleTemplateBinding {
            api_version: Some("management.cattle.io/v3".to_string()),
            kind: Some("ProjectRoleTemplateBinding".to_string()),
            metadata: Some(Box::new(IoK8sApimachineryPkgApisMetaV1ObjectMeta {
                name: Some("binding-id".to_string()),
                ..Default::default()
            })),
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
        assert_eq!(
            ioc.metadata.unwrap().name,
            Some("binding-id".to_string())
        );
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
