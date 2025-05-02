use serde::{Deserialize, Serialize};

use rancher_client::apis::{configuration::Configuration, management_cattle_io_v3_api::{read_management_cattle_io_v3_namespaced_project, ReadManagementCattleIoV3NamespacedProjectError}, Error, ResponseContent};
use reqwest::StatusCode;

use rancher_client::{
    apis::management_cattle_io_v3_api::{
        list_management_cattle_io_v3_namespaced_project,
        ListManagementCattleIoV3NamespacedProjectError,
    },
    models::{
        IoCattleManagementv3Project, IoCattleManagementv3ProjectList,
        IoCattleManagementv3ProjectSpecContainerDefaultResourceLimit,
        IoCattleManagementv3ProjectSpecNamespaceDefaultResourceQuota,
        IoCattleManagementv3ProjectSpecResourceQuotaLimit,
    },
};

/// Get all projects for a given namespace (cluster_id) from an endpoint using the provided configuration
/// 
/// # Arguments
///
/// * `configuration` - The configuration to use for the request
/// * `cluster_id` - The ID of the cluster (namespace) to get the projects for
///
/// # Returns
///
/// * `ListManagementCattleIoV3ProjectList` - The list of projects
///
/// # Errors
///
/// * `Error<ListManagementCattleIoV3NamespacedProjectError>` - The error that occurred while trying to get the projects
pub async fn get_projects(
    configuration: &Configuration,
    cluster_id: &str,
) -> Result<IoCattleManagementv3ProjectList, Error<ListManagementCattleIoV3NamespacedProjectError>>
{
    let result = list_management_cattle_io_v3_namespaced_project(
        configuration,
        cluster_id,
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
                    // Try to deserialize the content into IoCattleManagementv3ProjectList (Status200 case)
                    match serde_json::from_str(&response_content.content) {
                        Ok(data) => Ok(data),
                        Err(deserialize_err) => Err(Error::Serde(deserialize_err)),
                    }
                }
                _ => {
                    // If not status 200, treat as UnknownValue
                    match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(unknown_data) => Err(Error::ResponseError(ResponseContent {
                            status: response_content.status,
                            content: response_content.content,
                            entity: Some(
                                ListManagementCattleIoV3NamespacedProjectError::UnknownValue(
                                    unknown_data,
                                ),
                            ),
                        })),
                        Err(unknown_deserialize_err) => Err(Error::Serde(unknown_deserialize_err)),
                    }
                }
            }
        }
    }
}



/// Get a project by its ID
/// 
/// # Arguments
/// 
/// * `configuration` - The configuration to use for the request
/// * `cluster_id` - The ID of the cluster (namespace) to get the project for
/// * `project_id` - The ID of the project to get
/// # Returns
/// 
/// * `IoCattleManagementv3Project` - The project
/// # Errors
/// 
/// * `Error<ListManagementCattleIoV3NamespacedProjectError>` - The error that occurred while trying to get the project
/// 
pub async fn get_project(
    configuration: &Configuration,
    cluster_id: &str,
    project_id: &str,
) -> Result<IoCattleManagementv3Project, Error<ReadManagementCattleIoV3NamespacedProjectError>> {
    let result = read_management_cattle_io_v3_namespaced_project(
        configuration,
        project_id,
        cluster_id,
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
                    // Try to deserialize the content into IoCattleManagementv3Project (Status200 case)
                    match serde_json::from_str(&response_content.content) {
                        Ok(data) => Ok(data),
                        Err(deserialize_err) => Err(Error::Serde(deserialize_err)),
                    }
                }
                _ => {
                    // If not status 200, treat as UnknownValue
                    match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(unknown_data) => Err(Error::ResponseError(ResponseContent {
                            status: response_content.status,
                            content: response_content.content,
                            entity: Some(
                                ReadManagementCattleIoV3NamespacedProjectError::UnknownValue(
                                    unknown_data,
                                ),
                            ),
                        })),
                        Err(unknown_deserialize_err) => Err(Error::Serde(unknown_deserialize_err)),
                    }
                }
            }
        }
    }
}



#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Project {
    /// Name of the Kubernetes cluster this project belongs to.
    pub cluster_name: String,

    /// Unique project ID (typically the Kubernetes metadata.name).
    pub id: String,

    /// Human-readable description of the project.
    pub description: String,

    /// Default container resource limits applied within the project namespaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_default_resource_limit:
        Option<IoCattleManagementv3ProjectSpecContainerDefaultResourceLimit>,

    /// Human-readable display name for the project.
    pub display_name: String,

    /// Whether legacy Monitoring V1 is enabled (deprecated).
    pub enable_project_monitoring: bool,

    /// Default resource quotas applied at the namespace level.
    pub namespace_default_resource_quota:
        Option<IoCattleManagementv3ProjectSpecNamespaceDefaultResourceQuota>,

    /// Resource quota limits applied at the project level.
    pub resource_quota: Option<IoCattleManagementv3ProjectSpecResourceQuotaLimit>,
}

impl Project {
    pub fn new(
        cluster_name: String,
        id: String,
        display_name: String,
        description: String,
        container_default_resource_limit: Option<IoCattleManagementv3ProjectSpecContainerDefaultResourceLimit>,
        enable_project_monitoring: bool,
        namespace_default_resource_quota: Option<IoCattleManagementv3ProjectSpecNamespaceDefaultResourceQuota>,
        resource_quota: Option<IoCattleManagementv3ProjectSpecResourceQuotaLimit>,
    ) -> Self {
        Project {
            cluster_name,
            id,
            display_name,
            description,
            container_default_resource_limit,
            enable_project_monitoring,
            namespace_default_resource_quota,
            resource_quota,
        }
    }
}


// impl TryFrom<IoCattleManagementv3Project> for Project {
//     type Error = &'static str;

//     fn try_from(value: IoCattleManagementv3Project) -> Result<Self, Self::Error> {
//         let metadata: IoK8sApimachineryPkgApisMetaV1ObjectMeta =
//             *value.metadata.ok_or("missing metadata")?;

//         let spec: IoCattleManagementv3ProjectSpec = *value.spec.ok_or("missing spec")?;

//         let container_default_resource_limit = spec
//             .container_default_resource_limit;

//         let namespace_default_resource_quota = spec
//             .namespace_default_resource_quota;

//         // let resource_quota = resource_quota;

//         let resource_quota = match spec.resource_quota {
//             Some(ref quota) => Some(*quota.clone()),
//             None => None,
//         };

//         let resource_quota_limit = match resource_quota {
//             Some(quota) => quota.limit,
//             None => None,
//         };

//         Ok(Project {
//             cluster_name: spec.cluster_name,
//             id: metadata.name.ok_or("missing metadata.name")?,
//             description: spec.description.unwrap_or_default(),
//             container_default_resource_limit: container_default_resource_limit.as_deref().cloned(),
//             display_name: spec.display_name,
//             enable_project_monitoring: spec.enable_project_monitoring.unwrap_or(false),
//             namespace_default_resource_quota: namespace_default_resource_quota.as_deref().cloned(),
//             resource_quota: resource_quota_limit.as_deref().cloned(),
//         })
//     }
// }

impl TryFrom<IoCattleManagementv3Project> for Project {
    type Error = &'static str;

    fn try_from(value: IoCattleManagementv3Project) -> Result<Self, Self::Error> {
        let metadata = value.metadata.ok_or("missing metadata")?;
        let spec = value.spec.ok_or("missing spec")?;

        let container_default_resource_limit = spec
            .container_default_resource_limit
            .map(|b| *b);

        let namespace_default_resource_quota = spec
            .namespace_default_resource_quota
            .map(|b| *b);

        let resource_quota_limit = spec
        .resource_quota
        .and_then(|b| b.limit.map(|b| *b));
    
        

        Ok(Project {
            cluster_name: spec.cluster_name,
            id: metadata.name.ok_or("missing metadata.name")?,
            description: spec.description.unwrap_or_default(),
            container_default_resource_limit,
            display_name: spec.display_name,
            enable_project_monitoring: spec.enable_project_monitoring.unwrap_or(false),
            namespace_default_resource_quota,
            resource_quota: resource_quota_limit,
        })
    }
}

// impl TryFrom<IoCattleManagementv3Project> for Project {
//     type Error = &'static str;

//     fn try_from(value: IoCattleManagementv3Project) -> Result<Self, Self::Error> {
//         let metadata = value.metadata.ok_or("missing metadata")?;
//         let spec = value.spec.ok_or("missing spec")?;

//         let container_default_resource_limit = spec
//             .container_default_resource_limit
//             .map(|b| *b);

//         let namespace_default_resource_quota = spec
//             .namespace_default_resource_quota
//             .map(|b| *b);

//         let resource_quota = spec
//             .resource_quota
//             .and_then(|b| b.limit.map(|limit| *limit));

//         Project::new(
//             spec.cluster_name,
//             metadata.name.ok_or("missing metadata.name")?,
//             spec.display_name,
//             spec.description.unwrap_or_default(),
//             container_default_resource_limit,
//             spec.enable_project_monitoring.unwrap_or(false),
//             namespace_default_resource_quota,
//             resource_quota,
//         ).into()
//     }
// }
