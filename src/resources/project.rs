use std::path::Path;

use anyhow::{Context, Result};

use serde::{Deserialize, Serialize};
use serde_diff::SerdeDiff;

use similar::{ChangeTag, TextDiff};

use serde_json::Value;

use reqwest::StatusCode;

use rancher_client::{
    apis::{
        configuration::Configuration,
        management_cattle_io_v3_api::{
            create_management_cattle_io_v3_namespaced_project,
            delete_management_cattle_io_v3_namespaced_project,
            list_management_cattle_io_v3_namespaced_project,
            patch_management_cattle_io_v3_namespaced_project,
            read_management_cattle_io_v3_namespaced_project,
            DeleteManagementCattleIoV3NamespacedProjectError,
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
        IoK8sApimachineryPkgApisMetaV1Status,
    },
};
use tokio::fs::{metadata, read_to_string};
use tracing::{debug, error, info, trace, warn};

use crate::{
    deserialize_object,
    utils::file::{file_extension_from_format, FileFormat},
};

use crate::utils::logging::log_api_error;

use crate::models::ResourceVersionMatch;

use crate::utils::diff::diff_boxed_hashmap_string_string;

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
) -> Result<IoCattleManagementv3ProjectList> {
    let api_result = list_management_cattle_io_v3_namespaced_project(
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
    .await
    .context(format!("Failed to get projects for cluster {}", cluster_id));

    match api_result {
        Err(e) => {
            log_api_error("get_projects", &e);
            Err(e)
        }
        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");

            match response_content.status {
                StatusCode::OK => {
                    match serde_json::from_str::<IoCattleManagementv3ProjectList>(
                        &response_content.content,
                    ) {
                        Ok(data) => {
                            debug!(
                                "Successfully retrieved {} projects for cluster {}",
                                data.items.len(),
                                cluster_id
                            );
                            Ok(data)
                        }
                        Err(deserialize_err) => {
                            let err = anyhow::anyhow!(
                                "Failed to deserialize projects response: {}",
                                deserialize_err
                            );
                            log_api_error("get_projects:deserialize", &err);
                            Err(err)
                        }
                    }
                }
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized to get projects. Please check your credentials. Response: {}",
                        response_content.content
                    );
                    log_api_error("get_projects:unauthorized", &err);
                    Err(err)
                }
                StatusCode::NOT_FOUND => {
                    let err = anyhow::anyhow!(
                        "Cluster {} not found. Please check the cluster id. Response: {}",
                        cluster_id,
                        response_content.content
                    );
                    log_api_error("get_projects:not_found", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden to get projects. Please check your roles. Response: {}",
                        response_content.content
                    );
                    log_api_error("get_projects:forbidden", &err);
                    Err(err)
                }
                status => {
                    // For other status codes, try to parse error details from response
                    let err = match serde_json::from_str::<serde_json::Value>(
                        &response_content.content,
                    ) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when getting projects for cluster {}: {}", 
                                status, cluster_id, serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when getting projects for cluster {}: {}", 
                                status, cluster_id, response_content.content
                            )
                        }
                    };
                    log_api_error("get_projects:unexpected_status", &err);
                    Err(err)
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
/// * `resource_version` - The resource version to use for the request
/// # Returns
///
/// * `IoCattleManagementv3Project` - The project
/// # Errors
///
/// * `anyhow::Error` - The error that occurred while trying to get the project
///
#[async_backtrace::framed]
pub async fn find_project(
    configuration: &Configuration,
    cluster_id: &str,
    project_id: &str,
    resource_version: Option<&str>,
) -> Result<IoCattleManagementv3Project> {
    debug!(
        "Reading project with ID: {} from cluster: {}",
        project_id, cluster_id
    );

    let api_result = read_management_cattle_io_v3_namespaced_project(
        configuration,
        project_id,
        cluster_id,
        None,
        resource_version,
    )
    .await
    .context(format!(
        "Failed to find project with ID: {} in cluster: {}",
        project_id, cluster_id
    ));

    match api_result {
        Err(e) => {
            log_api_error("find_project", &e);
            Err(anyhow::anyhow!(e))
        }
        Ok(response_content) => {
            trace!(status = %response_content.status, "Received API response");

            match response_content.status {
                StatusCode::OK => {
                    match serde_json::from_str::<IoCattleManagementv3Project>(&response_content.content) {
                        Ok(data) => {
                            info!("Successfully found project with ID: {}", project_id);
                            Ok(data)
                        }
                        Err(deserialize_err) => {
                            let err = anyhow::anyhow!(
                                "Failed to deserialize project response: {}",
                                deserialize_err
                            );
                            log_api_error("find_project:deserialize", &err);
                            Err(err)
                        }
                    }
                }
                StatusCode::NOT_FOUND => {
                    let err = anyhow::anyhow!(
                        "Project with ID: {} not found. Response: {}",
                        project_id,
                        response_content.content
                    );
                    log_api_error("find_project:not_found", &err);
                    Err(err)
                }
                StatusCode::UNAUTHORIZED => {
                    let err = anyhow::anyhow!(
                        "Unauthorized access while trying to find project with ID: {}. Response: {}",
                        project_id,
                        response_content.content
                    );
                    log_api_error("find_project:unauthorized", &err);
                    Err(err)
                }
                StatusCode::FORBIDDEN => {
                    let err = anyhow::anyhow!(
                        "Forbidden access while trying to find project with ID: {}. Response: {}",
                        project_id,
                        response_content.content
                    );
                    log_api_error("find_project:forbidden", &err);
                    Err(err)
                }
                status => {
                    let err = match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(error_obj) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when finding project with ID: {}: {}",
                                status, project_id, serde_json::to_string_pretty(&error_obj).unwrap_or_else(|_| response_content.content.clone())
                            )
                        }
                        Err(_) => {
                            anyhow::anyhow!(
                                "Unexpected status code {} when finding project with ID: {}: {}",
                                status, project_id, response_content.content
                            )
                        }
                    };
                    log_api_error("find_project:unexpected_status", &err);
                    Err(err)
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
/// * `anyhow::Error` - The error that occurred while trying to patch the project
///
#[async_backtrace::framed]
pub async fn update_project(
    configuration: &Configuration,
    cluster_id: &str,
    project_id: &str,
    patch_value: Value,
) -> Result<IoCattleManagementv3Project> {
    info!(
        "Patching project with ID: {} in cluster: {}",
        project_id, cluster_id
    );

    let patch_array = match patch_value {
        Value::Array(arr) => arr,
        Value::Null => {
            error!("Expected patch to serialize to a JSON array, but got null");
            return Err(anyhow::anyhow!(
                "Expected patch to serialize to a JSON array, but got null"
            ));
        }
        _ => {
            error!(
                "Expected patch to serialize to a JSON array, but got: {:?}",
                patch_value
            );
            return Err(anyhow::anyhow!("Expected patch to serialize to a JSON array"));
        }
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
        Err(e) => {
            error!("Failed to patch project: {}", e);
            Err(anyhow::anyhow!(e))
        }
        Ok(response_content) => match response_content.status {
            StatusCode::OK => {
                info!("Successfully patched project with ID: {}", project_id);
                match serde_json::from_str(&response_content.content) {
                    Ok(data) => Ok(data),
                    Err(deserialize_err) => {
                        error!("Failed to deserialize response: {}", deserialize_err);
                        Err(anyhow::anyhow!(deserialize_err))
                    }
                }
            }
            StatusCode::NOT_FOUND => {
                error!("Project with ID: {} not found", project_id);
                Err(anyhow::anyhow!("Project with ID: {} not found", project_id))
            }
            StatusCode::UNAUTHORIZED => {
                error!(
                    "Unauthorized access while trying to patch project with ID: {}",
                    project_id
                );
                Err(anyhow::anyhow!(
                    "Unauthorized access while trying to patch project with ID: {}",
                    project_id
                ))
            }
            StatusCode::BAD_REQUEST => {
                error!(
                    "Bad request when updating project with ID: {}. Request body was: {}",
                    project_id, response_content.content
                );
                Err(anyhow::anyhow!(
                    "Bad request when updating project with ID: {}. Request body was: {}",
                    project_id, response_content.content
                ))
            }
            StatusCode::FORBIDDEN => {
                error!("Forbidden to patch project with ID: {}", project_id);
                Err(anyhow::anyhow!("Forbidden to patch project with ID: {}", project_id))
            }
            _ => {
                error!(
                    "Received unexpected status code: {}",
                    response_content.status
                );
                Err(anyhow::anyhow!("Received unexpected status code: {}", response_content.status))
            }
        },
    }
}

/// Create a project from a given configuration
/// # Arguments
/// * `configuration` - The configuration for the request
/// * `cluster_id` - The ID of the cluster to create the project in
/// * `body` - The project to create
/// # Returns
/// * `IoCattleManagementv3Project` - The project that was created
/// # Errors
/// * `anyhow::Error` - The error that occurred during the request
///
#[async_backtrace::framed]
pub async fn create_project(
    configuration: &Configuration,
    cluster_id: &str,
    body: IoCattleManagementv3Project,
) -> Result<IoCattleManagementv3Project> {
    let project_id = body.metadata.as_ref().unwrap().name.clone().unwrap();
    info!(
        "Creating project in cluster: {} with ID: {}",
        cluster_id, project_id
    );

    let result = create_management_cattle_io_v3_namespaced_project(
        configuration,
        cluster_id,
        body,
        None,
        None,
        Some(crate::FULL_CLIENT_ID),
        None,
    )
    .await;

    match result {
        Ok(response_content) => {
            info!("Successfully created project with ID: {}", project_id);
            match serde_json::from_str(&response_content.content) {
                Ok(data) => Ok(data),
                Err(deserialize_err) => {
                    error!("Failed to deserialize response: {}", deserialize_err);
                    Err(anyhow::anyhow!(deserialize_err))
                }
            }
        }
        Err(Error::ResponseError(resp)) => {
            match resp.status {
                StatusCode::CONFLICT => {
                    warn!("Failed to create project, conflict occurred");
                    Err(anyhow::anyhow!("Project creation conflict: {}", resp.content))
                }
                StatusCode::UNAUTHORIZED => {
                    error!("Unauthorized to create project");
                    Err(anyhow::anyhow!("Unauthorized to create project"))
                }
                StatusCode::FORBIDDEN => {
                    error!("Forbidden to create project");
                    Err(anyhow::anyhow!("Forbidden to create project"))
                }
                _ => {
                    error!("API error: {} - {}", resp.status, resp.content);
                    Err(anyhow::anyhow!("API error: {} - {}", resp.status, resp.content))
                }
            }
        }
        Err(e) => {
            error!("Request error: {}", e);
            Err(anyhow::anyhow!("Request error: {}", e))
        }
    }
}

/// load a specific project configuration from the base path
///
/// # Arguments
/// `base_path`: The base path to load the project from
/// `cluster_id`: The cluster ID to load the project from
/// `project_id`: The project ID to load the project from
/// `file_format`: The file format to load the project from
///
/// # Returns
/// `Project`: The project object
///
#[async_backtrace::framed]
pub async fn load_project(
    base_path: &Path,
    endpoint_url: &str,
    cluster_id: &str,
    project_name: &str,
    file_format: FileFormat,
) -> Result<Project, Box<dyn std::error::Error>> {
    // create the path to the project
    let project_path = base_path
        .join(endpoint_url.replace("https://", "").replace("/", "_"))
        .join(cluster_id)
        .join(project_name);
    // check if the path exists

    metadata(&project_path)
        .await
        .map_err(|_| format!("Project path does not exist: {:?}", project_path))?
        .is_dir()
        .then_some(())
        .ok_or_else(|| format!("Not a directory: {:?}", project_path))?;

    // build the file path
    let project_file = project_path.join(format!(
        "{}.{}",
        project_name,
        file_extension_from_format(&file_format)
    ));

    // ensure the file exists
    metadata(&project_file)
        .await
        .map_err(|_| format!("Project file does not exist: {:?}", project_file))?
        .is_file()
        .then_some(())
        .ok_or_else(|| format!("Not a file: {:?}", project_file))?;

    // read and deserialize
    let content = read_to_string(&project_file)
        .await
        .map_err(|e| format!("Failed to read file {:?}: {}", project_file, e))?;

    Ok(deserialize_object(&content, &file_format)?)
}

/// Delete a project by its ID  
/// # Arguments  
/// * `configuration` - The configuration to use for the request  
/// * `cluster_id` - The ID of the cluster (namespace) containing the project  
/// * `project_id` - The ID of the project to delete  
/// # Returns  
/// * `IoCattleManagementv3Project` - The deleted project  
/// # Errors  
/// * `Error<DeleteManagementCattleIoV3NamespacedProjectError>` - The error that occurred while trying to delete the project  
#[async_backtrace::framed]
pub async fn delete_project(
    configuration: &Configuration,
    cluster_id: &str,
    project_id: &str,
) -> Result<
    Result<IoCattleManagementv3Project, IoK8sApimachineryPkgApisMetaV1Status>,
    Error<DeleteManagementCattleIoV3NamespacedProjectError>,
> {
    info!(
        "Deleting project with ID: {} in cluster: {}",
        project_id, cluster_id
    );
    let result = delete_management_cattle_io_v3_namespaced_project(
        configuration,
        project_id,
        cluster_id,
        None, // pretty
        None, // dry_run
        None, // grace_period_seconds
        None, // orphan_dependents
        None, // propagation_policy
        None, // body
    )
    .await;

    match result {
        Err(e) => {
            error!("Failed to delete project: {}", e);
            return Err(e);
        }
        Ok(response_content) => {
            trace!("Response: {}", response_content.content);
            match response_content.status {
                StatusCode::OK => {
                    info!("Successfully deleted project with ID: {}", project_id);
                    // Try to deserialize as Project first
                    match serde_json::from_str::<IoCattleManagementv3Project>(
                        &response_content.content,
                    ) {
                        Ok(project) => Ok(Ok(project)),
                        Err(_) => {
                            // If that fails, try to deserialize as Status
                            match serde_json::from_str::<IoK8sApimachineryPkgApisMetaV1Status>(
                                &response_content.content,
                            ) {
                                Ok(status) => Ok(Err(status)),
                                Err(deserialize_err) => {
                                    error!("Failed to deserialize response as either Project or Status: {}", deserialize_err);
                                    Err(Error::Serde(deserialize_err))
                                }
                            }
                        }
                    }
                }
                StatusCode::NOT_FOUND => {
                    error!("Project with ID: {} not found", project_id);
                    Err(Error::ResponseError(ResponseContent {
                        status: response_content.status,
                        content: response_content.content,
                        entity: None,
                    }))
                }
                StatusCode::UNAUTHORIZED => {
                    error!("Unauthorized to delete project with ID: {}", project_id);
                    Err(Error::ResponseError(ResponseContent {
                        status: response_content.status,
                        content: response_content.content,
                        entity: None,
                    }))
                }
                StatusCode::FORBIDDEN => {
                    error!("Forbidden to delete project with ID: {}", project_id);
                    Err(Error::ResponseError(ResponseContent {
                        status: response_content.status,
                        content: response_content.content,
                        entity: None,
                    }))
                }
                _ => {
                    error!(
                        "Received unexpected status code: {}",
                        response_content.status
                    );
                    let unknown_data =
                        match serde_json::from_str::<Value>(&response_content.content) {
                            Ok(val) => val,
                            Err(deserialize_err) => {
                                error!(
                                    "Failed to deserialize unknown response content: {}",
                                    deserialize_err
                                );
                                return Err(Error::Serde(deserialize_err));
                            }
                        };
                    Err(Error::ResponseError(ResponseContent {
                        status: response_content.status,
                        content: response_content.content,
                        entity: Some(
                            DeleteManagementCattleIoV3NamespacedProjectError::UnknownValue(
                                unknown_data,
                            ),
                        ),
                    }))
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
    pub id: Option<String>,

    /// Human-readable description of the project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace_default_resource_quota:
        Option<IoCattleManagementv3ProjectSpecNamespaceDefaultResourceQuota>,

    /// Resource quota limits applied at the project level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_quota: Option<IoCattleManagementv3ProjectSpecResourceQuotaLimit>,
}

impl Project {
    pub fn new(
        annotations: Option<std::collections::HashMap<String, String>>,
        cluster_name: String,
        container_default_resource_limit: Option<
            IoCattleManagementv3ProjectSpecContainerDefaultResourceLimit,
        >,
        description: Option<String>,
        display_name: String,
        enable_project_monitoring: Option<bool>,
        id: Option<String>,
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
    type Error = anyhow::Error;

    fn try_from(value: IoCattleManagementv3Project) -> Result<Self, Self::Error> {
        let metadata = value
            .metadata
            .ok_or_else(|| anyhow::anyhow!("Missing metadata field"))?;
        
        let spec = value
            .spec
            .ok_or_else(|| anyhow::anyhow!("Missing spec field"))?;

        let container_default_resource_limit = spec.container_default_resource_limit;
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
            description: spec.description,
            display_name: spec.display_name,
            enable_project_monitoring: spec.enable_project_monitoring,
            id: metadata.name,
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
    type Error = anyhow::Error;

    fn try_from(value: Project) -> Result<Self, Self::Error> {
        // Construct metadata
        let metadata = IoK8sApimachineryPkgApisMetaV1ObjectMeta {
            name: value.id,
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
            description: value.description.clone(),
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

        metadata.name == other.id
            && spec.cluster_name == other.cluster_name
            && spec.description == other.description
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
            description: Some("Test project".to_string()),
            display_name: "Project One".to_string(),
            enable_project_monitoring: Some(true),
            id: Some("proj-1".to_string()),
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

        project.description = Some("Changed".to_string());

        assert_ne!(rancher_project, project);
        assert_ne!(project, rancher_project);
    }
}
