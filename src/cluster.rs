use serde::{Deserialize, Serialize};

use rancher_client::apis::{configuration::Configuration, Error, ResponseContent};
use reqwest::StatusCode;

use rancher_client::{
    apis::management_cattle_io_v3_api::{
        list_management_cattle_io_v3_clusters, ListManagementCattleIoV3ClustersError,
    },
    models::{
        IoCattleManagementv3Cluster, IoCattleManagementv3ClusterList,
        IoCattleManagementv3ClusterSpec, IoK8sApimachineryPkgApisMetaV1ObjectMeta,
    },
};

/// Get all clusters from an endpoint using the provided configuration
///
/// # Arguments
///
/// * `configuration` - The configuration to use for the request
///
/// # Returns
///
/// * `IoCattleManagementv3ClusterList` - The list of clusters
///
/// # Errors
///
/// * `Error<ListManagementCattleIoV3ClustersError>` - The error that occurred while trying to get the clusters
///
pub async fn get_clusters(
    configuration: &Configuration,
) -> Result<IoCattleManagementv3ClusterList, Error<ListManagementCattleIoV3ClustersError>> {
    let result = list_management_cattle_io_v3_clusters(
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
                    // Try to deserialize the content into IoCattleManagementv3ClusterList (Status200 case)
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
                                entity: Some(ListManagementCattleIoV3ClustersError::UnknownValue(
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

#[derive(Serialize, Deserialize)]
pub struct Cluster {
    id: String,
    display_name: String,
    description: String,
}

impl Cluster {
    pub fn new(id: String, name: String, description: String) -> Self {
        Cluster {
            id,
            display_name: name,
            description,
        }
    }
}

// impl From<IoCattleManagementv3Cluster> for Cluster {
//     fn from(cluster: IoCattleManagementv3Cluster) -> Self {
//         Cluster {
//             id: cluster.metadata.unwrap_or_default().name.unwrap_or_default(),
//             display_name: cluster.spec.display_name,
//             description: cluster.spec.description.unwrap_or_else(|| "".to_string()),
//         }
//     }
// }

impl TryFrom<IoCattleManagementv3Cluster> for Cluster {
    type Error = &'static str;

    fn try_from(value: IoCattleManagementv3Cluster) -> Result<Self, Self::Error> {
        let metadata: IoK8sApimachineryPkgApisMetaV1ObjectMeta =
            *value.metadata.ok_or("missing metadata")?;
        let spec: IoCattleManagementv3ClusterSpec = *value.spec;

        Ok(Cluster {
            id: metadata.name.ok_or("missing name")?,
            display_name: spec.display_name,
            description: spec.description.unwrap_or_else(|| "".to_string()),
        })
    }
}
