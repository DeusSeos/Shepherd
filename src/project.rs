
use serde::{Deserialize, Serialize};

use rancher_client::{apis::{configuration::Configuration, management_cattle_io_v3_api::{read_management_cattle_io_v3_namespaced_project, ReadManagementCattleIoV3NamespacedProjectError}, Error, ResponseContent}, models::{self, IoCattleManagementv3ProjectSpec, IoK8sApimachineryPkgApisMetaV1ObjectMeta}};
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

    // annotations: Option<std::collections::HashMap<String, String>>,
    /// Annotations applied to the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<std::collections::HashMap<String, String>>,

    /// Labels applied to the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<std::collections::HashMap<String, String>>,

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
        annotations: Option<std::collections::HashMap<String, String>>,
        cluster_name: String,
        container_default_resource_limit: Option<IoCattleManagementv3ProjectSpecContainerDefaultResourceLimit>,
        description: String,
        display_name: String,
        enable_project_monitoring: bool,
        id: String,
        labels: Option<std::collections::HashMap<String, String>>,
        namespace_default_resource_quota: Option<IoCattleManagementv3ProjectSpecNamespaceDefaultResourceQuota>,
        resource_quota: Option<IoCattleManagementv3ProjectSpecResourceQuotaLimit>,
    ) -> Self {
        Project {
            annotations,
            cluster_name,
            container_default_resource_limit,
            description,
            display_name,
            enable_project_monitoring,
            id,
            labels,
            namespace_default_resource_quota,
            resource_quota,
        }
    }
}

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
            annotations: metadata.annotations,
            labels: metadata.labels,
            cluster_name: spec.cluster_name,
            container_default_resource_limit,
            description: spec.description.unwrap_or_default(),
            display_name: spec.display_name,
            enable_project_monitoring: spec.enable_project_monitoring.unwrap_or(false),
            id: metadata.name.ok_or("missing metadata.name")?,
            namespace_default_resource_quota,
            resource_quota: resource_quota_limit,
        })
    }
}


impl TryFrom<Project> for IoCattleManagementv3Project {
    type Error = &'static str;

    fn try_from(value: Project) -> Result<Self, Self::Error> {
        // Construct metadata
        let metadata = IoK8sApimachineryPkgApisMetaV1ObjectMeta {
            name: Some(value.id.clone()),
            annotations: value.annotations.clone(),
            labels: value.labels.clone(),
            ..Default::default()
        };

        // Construct spec
        let spec = IoCattleManagementv3ProjectSpec {
            cluster_name: value.cluster_name.clone(),
            description: Some(value.description.clone()),
            display_name: value.display_name.clone(),
            enable_project_monitoring: Some(value.enable_project_monitoring),
            container_default_resource_limit: value.container_default_resource_limit.clone().map(Box::new),
            namespace_default_resource_quota: value.namespace_default_resource_quota.clone().map(Box::new),
            resource_quota: value.resource_quota.clone().map(|rq| {
                Box::new(models::IoCattleManagementv3ProjectSpecResourceQuota {
                    limit: Some(Box::new(rq)),
                    ..Default::default()
                })
            }),
            ..Default::default()
        };

        Ok(IoCattleManagementv3Project {
            api_version: Some("management.cattle.io/v3".to_string()),
            kind: Some("Project".to_string()),
            metadata: Some(Box::new(metadata)),
            spec: Some(Box::new(spec)),
            status: None,
        })
    }
}


impl PartialEq<Project> for IoCattleManagementv3Project {
    fn eq(&self, other: &Project) -> bool {
        let metadata = match &self.metadata {
            Some(m) => m,
            None => return false,
        };

        let spec = match &self.spec {
            Some(s) => s,
            None => return false,
        };

        let resource_quota_limit = spec.resource_quota
            .as_ref()
            .and_then(|rq| rq.limit.as_deref());
        

        let container_limit = spec.container_default_resource_limit.as_deref();
        let namespace_quota = spec.namespace_default_resource_quota.as_deref();

        metadata.name.as_deref() == Some(&other.id)
            && spec.cluster_name == other.cluster_name
            && spec.description.as_deref().unwrap_or_default() == other.description
            && spec.display_name == other.display_name
            && spec.enable_project_monitoring.unwrap_or(false) == other.enable_project_monitoring
            && metadata.annotations == other.annotations
            && metadata.labels == other.labels
            && container_limit == other.container_default_resource_limit.as_ref()
            && namespace_quota == other.namespace_default_resource_quota.as_ref()
            && resource_quota_limit == other.resource_quota.as_ref()
    }
}


impl PartialEq<IoCattleManagementv3Project> for Project {
    fn eq(&self, other: &IoCattleManagementv3Project) -> bool {
        other == self
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    
    fn sample_project() -> Project {
        Project {
            cluster_name: "cluster-1".to_string(),
            id: "proj-1".to_string(),
            description: "Test project".to_string(),
            annotations: Some(std::collections::HashMap::new()),
            labels: Some(std::collections::HashMap::new()),
            container_default_resource_limit: None,
            display_name: "Project One".to_string(),
            enable_project_monitoring: true,
            namespace_default_resource_quota: None,
            resource_quota: None,
        }
    }

    fn sample_iocattle_project() -> IoCattleManagementv3Project {
        IoCattleManagementv3Project {
            metadata: Some(Box::new(models::IoK8sApimachineryPkgApisMetaV1ObjectMeta {
                name: Some("proj-1".to_string()),
                annotations: Some(std::collections::HashMap::new()),
                labels: Some(std::collections::HashMap::new()),
                ..Default::default()
            })),
            spec: Some(Box::new(models::IoCattleManagementv3ProjectSpec {
                cluster_name: "cluster-1".to_string(),
                description: Some("Test project".to_string()),
                display_name: "Project One".to_string(),
                enable_project_monitoring: Some(true),
                container_default_resource_limit: None,
                namespace_default_resource_quota: None,
                resource_quota: None,
                ..Default::default()
            })),
            ..Default::default()
        }
    }

    #[test]
    fn test_eq_both_directions() {
        let project = sample_project();
        let rancher_project = sample_iocattle_project();

        assert_eq!(rancher_project, project);
        assert_eq!(project, rancher_project); // requires the reverse impl
    }

    #[test]
    fn test_inequality() {
        let mut project = sample_project();
        let rancher_project = sample_iocattle_project();

        project.description = "Changed".to_string();

        assert_ne!(rancher_project, project);
        assert_ne!(project, rancher_project);
    }
}
