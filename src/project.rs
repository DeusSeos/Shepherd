use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

use similar::{ChangeTag, TextDiff};

use serde_json::Value;

use reqwest::StatusCode;

use rancher_client::{
    apis::{
        configuration::Configuration,
        management_cattle_io_v3_api::{
            list_management_cattle_io_v3_namespaced_project,
            patch_management_cattle_io_v3_namespaced_project,
            read_management_cattle_io_v3_namespaced_project,
            ListManagementCattleIoV3NamespacedProjectError,
            PatchManagementCattleIoV3NamespacedProjectError,
            ReadManagementCattleIoV3NamespacedProjectError,
        },
        Error, ResponseContent,
    },
    models::{
        self, IoCattleManagementv3Project, IoCattleManagementv3ProjectList,
        IoCattleManagementv3ProjectSpec,
        IoCattleManagementv3ProjectSpecContainerDefaultResourceLimit,
        IoCattleManagementv3ProjectSpecNamespaceDefaultResourceQuota,
        IoCattleManagementv3ProjectSpecResourceQuotaLimit,
        IoK8sApimachineryPkgApisMetaV1ObjectMeta, IoK8sApimachineryPkgApisMetaV1Patch,
    },
};

use crate::ResourceVersionMatch;

use crate::diff::diff_boxed_hashmap_string_string;

pub const PROJECT_EXCLUDE_PATHS: &[&str] = &[
    "metadata.creationTimestamp",
    "metadata.finalizers",
    "metadata.generateName",
    "metadata.generation",
    "metadata.managedFields",
    "metadata.resourceVersion",
    "spec.resourceQuota.usedLimit",
    "status",
];

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
#[async_backtrace::framed]
pub async fn get_projects(
    configuration: &Configuration,
    cluster_id: &str,
    field_selector: Option<&str>,
    label_selector: Option<&str>,
    limit: Option<i32>,
    resource_version: Option<&str>,
    resource_version_match: Option<ResourceVersionMatch>,
    continue_: Option<&str>,
) -> Result<IoCattleManagementv3ProjectList, Error<ListManagementCattleIoV3NamespacedProjectError>>
{
    let result = list_management_cattle_io_v3_namespaced_project(
        configuration,
        cluster_id,
        None,
        None,
        continue_,
        field_selector,
        label_selector,
        limit,
        resource_version,
        resource_version_match.map(|v| v.as_str()),
        None,
        None,
        None,
    )
    .await;

    match result {
        Err(e) => {
            // TODO: Handle specific error cases
            Err(e)
        }
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

/// Find a project by its ID
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
#[async_backtrace::framed]
pub async fn find_project(
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

/// Update a project by its ID
/// # Arguments
/// * `configuration` - The configuration to use for the request
/// * `cluster_id` - The ID of the cluster (namespace) to get the project for
/// * `project_id` - The ID of the project to get
/// * `body` - The Kubernetes patch body to apply
/// # Returns
/// * `IoCattleManagementv3Project` - The project
/// # Errors
/// * `Error<PatchManagementCattleIoV3NamespacedProjectError>` - The error that occurred while trying to patch the project
///
#[async_backtrace::framed]
pub async fn update_project(
    configuration: &Configuration,
    cluster_id: &str,
    project_id: &str,
    patch_value: Value,
) -> Result<IoCattleManagementv3Project, Error<PatchManagementCattleIoV3NamespacedProjectError>> {
    let patch_array = match patch_value {
        Value::Array(arr) => arr,
        _ => panic!("Expected patch to serialize to a JSON array"),
    };

    let k8s_patch = IoK8sApimachineryPkgApisMetaV1Patch::Array(patch_array);

    let result = patch_management_cattle_io_v3_namespaced_project(
        configuration,
        project_id,
        cluster_id,
        Some(k8s_patch),
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
                                PatchManagementCattleIoV3NamespacedProjectError::UnknownValue(
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

#[derive(Serialize, Deserialize, SerdeDiff, Debug, Clone, PartialEq)]
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

    /// The namespace in which the project is created. (Should be the same as the cluster name)
    pub namespace: String,

    /// An opaque value that represents the internal version of this object that can be used by clients to determine when objects have changed. May be used for optimistic concurrency, change detection, and the watch operation on a resource or set of resources. Clients must treat these values as opaque and passed unmodified back to the server. They may only be valid for a particular resource or set of resources.  Populated by the system. Read-only. Value must be treated as opaque by clients and . More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#concurrency-control-and-consistency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,

    /// The UID of the project. This cannot be changed. Rancher will set this value when the project is created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,

    /// Whether legacy Monitoring V1 is enabled (deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_project_monitoring: Option<bool>,

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
        container_default_resource_limit: Option<
            IoCattleManagementv3ProjectSpecContainerDefaultResourceLimit,
        >,
        description: String,
        display_name: String,
        enable_project_monitoring: Option<bool>,
        id: String,
        labels: Option<std::collections::HashMap<String, String>>,
        namespace_default_resource_quota: Option<
            IoCattleManagementv3ProjectSpecNamespaceDefaultResourceQuota,
        >,
        namespace: String,
        resource_quota: Option<IoCattleManagementv3ProjectSpecResourceQuotaLimit>,
        resource_version: Option<String>,
        uid: Option<String>,
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
            namespace,
            resource_quota,
            resource_version,
            uid,
        }
    }
}

impl TryFrom<IoCattleManagementv3Project> for Project {
    type Error = &'static str;

    fn try_from(value: IoCattleManagementv3Project) -> Result<Self, Self::Error> {
        let metadata = value.metadata.ok_or("missing metadata")?;
        let spec = value.spec.ok_or("missing spec")?;

        // let container_default_resource_limit = spec
        //     .container_default_resource_limit
        //     .map(|b| *b);

        let container_default_resource_limit = spec.container_default_resource_limit;

        // let namespace_default_resource_quota = spec
        //     .namespace_default_resource_quota
        //     .map(|b| *b);

        let namespace_default_resource_quota = spec.namespace_default_resource_quota;

        let resource_quota_limit = spec.resource_quota.and_then(|b| b.limit.map(|b| *b));

        let annotations = metadata.annotations.map(|a| {
            a.into_iter()
                .collect::<std::collections::HashMap<String, String>>()
        });

        let labels = metadata.labels.map(|a| {
            a.into_iter()
                .collect::<std::collections::HashMap<String, String>>()
        });

        Ok(Project {
            annotations,
            cluster_name: spec.cluster_name,
            container_default_resource_limit,
            description: spec.description.unwrap_or_default(),
            display_name: spec.display_name,
            enable_project_monitoring: spec.enable_project_monitoring,
            id: metadata.name.ok_or("missing metadata.name")?,
            labels,
            namespace_default_resource_quota,
            namespace: metadata.namespace.unwrap_or_default(),
            resource_quota: resource_quota_limit,
            resource_version: metadata.resource_version,
            uid: metadata.uid,
        })
    }
}

impl TryFrom<Project> for IoCattleManagementv3Project {
    type Error = &'static str;

    fn try_from(value: Project) -> Result<Self, Self::Error> {
        // Construct metadata
        let metadata = IoK8sApimachineryPkgApisMetaV1ObjectMeta {
            name: Some(value.id.clone()),
            annotations: value.annotations.clone().map(|a| {
                a.into_iter()
                    .collect::<std::collections::HashMap<String, String>>()
            }),
            labels: value.labels.clone().map(|a| {
                a.into_iter()
                    .collect::<std::collections::HashMap<String, String>>()
            }),
            namespace: Some(value.namespace.clone()),
            resource_version: value.resource_version.clone(),
            uid: value.uid.clone(),
            ..Default::default()
        };

        // Construct spec
        let spec = IoCattleManagementv3ProjectSpec {
            cluster_name: value.cluster_name.clone(),
            description: Some(value.description.clone()),
            display_name: value.display_name.clone(),
            enable_project_monitoring: value.enable_project_monitoring,
            // container_default_resource_limit: value.container_default_resource_limit.clone().map(Box::new),
            // namespace_default_resource_quota: value.namespace_default_resource_quota.clone().map(Box::new),
            container_default_resource_limit: value.container_default_resource_limit.clone(),
            namespace_default_resource_quota: value.namespace_default_resource_quota.clone(),
            // resource_quota: value.resource_quota.clone().map(|rq| {
            //     Box::new(models::IoCattleManagementv3ProjectSpecResourceQuota {
            //         limit: Some(Box::new(rq)),
            //         ..Default::default()
            //     })
            // }),
            resource_quota: value.resource_quota.clone().map(|rq| {
                models::IoCattleManagementv3ProjectSpecResourceQuota {
                    limit: Some(Box::new(rq)),
                    ..Default::default()
                }
            }),
            ..Default::default()
        };

        Ok(IoCattleManagementv3Project {
            api_version: Some("management.cattle.io/v3".to_string()),
            kind: Some("Project".to_string()),
            // metadata: Some(Box::new(metadata)),
            metadata: Some(metadata),
            // spec: Some(Box::new(spec)),
            spec: Some(spec),
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

        if metadata.resource_version != other.resource_version {
            return false;
        }

        let spec = match &self.spec {
            Some(s) => s,
            None => return false,
        };

        let resource_quota_limit = spec
            .resource_quota
            .as_ref()
            .and_then(|rq| rq.limit.as_deref());

        // let container_limit = spec.container_default_resource_limit.as_deref();
        // let namespace_quota = spec.namespace_default_resource_quota.as_deref();
        let container_limit = spec.container_default_resource_limit.as_ref();
        let namespace_quota = spec.namespace_default_resource_quota.as_ref();

        let annotations = other.annotations.clone().map(|a| {
            a.into_iter()
                .collect::<std::collections::HashMap<String, String>>()
        });

        let labels = other.labels.clone().map(|a| {
            a.into_iter()
                .collect::<std::collections::HashMap<String, String>>()
        });

        metadata.name.as_deref() == Some(&other.id)
            && spec.cluster_name == other.cluster_name
            && spec.description.as_deref().unwrap_or_default() == other.description
            && spec.display_name == other.display_name
            && spec.enable_project_monitoring == other.enable_project_monitoring
            && metadata.annotations == annotations
            && metadata.labels == labels
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

pub fn show_text_diff(project_a: &Project, project_b: &IoCattleManagementv3Project) {
    // convert the project to IoCattleManagementv3Project
    let rancher_project_a: IoCattleManagementv3Project = project_a.clone().try_into().unwrap();

    // convert project a to json
    let project_a_json = serde_json::to_string_pretty(&rancher_project_a).unwrap();
    println!("Project A: {}", project_a_json);
    // convert project b to json
    let project_b_json = serde_json::to_string_pretty(&project_b).unwrap();
    println!("Project B: {}", project_b_json);

    let diff = TextDiff::from_lines(&project_a_json, &project_b_json);
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        print!("{}{}", sign, change);
    }
}

/// Show the diff between two projects
///
/// # Arguments
/// /// * `project_a` - The first project
/// * `project_b` - The second project
/// # Returns
/// * `()` - Nothing
/// # Errors
pub fn show_project_diff(project_a: &Project, project_b: &IoCattleManagementv3Project) {
    // convert the project to IoCattleManagementv3Project
    let rancher_project_a: IoCattleManagementv3Project = project_a.clone().try_into().unwrap();

    let diff = serde_diff::Diff::serializable(&rancher_project_a, project_b);

    let diff_json = serde_json::to_string_pretty(&diff).unwrap();
    println!("Project diff:\n{}", diff_json);

    println!("\nAnnotation‑level diff:");
    diff_boxed_hashmap_string_string(
        rancher_project_a
            .metadata
            .as_ref()
            .and_then(|m| m.annotations.as_ref()),
        project_b
            .metadata
            .as_ref()
            .and_then(|m| m.annotations.as_ref()),
    );

    println!("\nLabel‑level diff:");
    diff_boxed_hashmap_string_string(
        rancher_project_a
            .metadata
            .as_ref()
            .and_then(|m| m.labels.as_ref()),
        project_b.metadata.as_ref().and_then(|m| m.labels.as_ref()),
    );
}


#[cfg(test)]
mod tests {
    use super::*;

    fn sample_project() -> Project {
        Project {
            annotations: Some(std::collections::HashMap::new()),
            cluster_name: "cluster-1".to_string(),
            container_default_resource_limit: None,
            description: "Test project".to_string(),
            display_name: "Project One".to_string(),
            enable_project_monitoring: Some(true),
            id: "proj-1".to_string(),
            labels: Some(std::collections::HashMap::new()),
            namespace_default_resource_quota: None,
            namespace: "cluster-1".to_string(),
            resource_quota: None,
            resource_version: Some("5555".to_string()),
            uid: Some("1234".to_string()),
        }
    }

    fn sample_iocattle_project() -> IoCattleManagementv3Project {
        IoCattleManagementv3Project {
            metadata: Some(models::IoK8sApimachineryPkgApisMetaV1ObjectMeta {
                annotations: Some(std::collections::HashMap::new()),
                labels: Some(std::collections::HashMap::new()),
                name: Some("proj-1".to_string()),
                namespace: Some("cluster-1".to_string()),
                resource_version: Some("5555".to_string()),
                uid: Some("1234".to_string()),
                ..Default::default()
            }),
            spec: Some(models::IoCattleManagementv3ProjectSpec {
                cluster_name: "cluster-1".to_string(),
                container_default_resource_limit: None,
                description: Some("Test project".to_string()),
                display_name: "Project One".to_string(),
                enable_project_monitoring: Some(true),
                namespace_default_resource_quota: None,
                resource_quota: None,
            }),
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
