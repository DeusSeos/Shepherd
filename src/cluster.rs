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
#[async_backtrace::framed]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Cluster {
    pub id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Cluster {
    pub fn new(id: String, name: String, description: Option<String>) -> Self {
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
            value.metadata.ok_or("missing metadata")?;
        let spec: IoCattleManagementv3ClusterSpec = *value.spec;

        Ok(Cluster {
            id: metadata.name.ok_or("missing name")?,
            display_name: spec.display_name,
            description: spec.description,
        })
    }
}


impl TryFrom<Cluster> for IoCattleManagementv3Cluster {
    type Error = &'static str;

    fn try_from(value: Cluster) -> Result<Self, Self::Error> {
        let metadata = IoK8sApimachineryPkgApisMetaV1ObjectMeta {
            name: Some(value.id),
            ..Default::default()
        };
        let spec = IoCattleManagementv3ClusterSpec {
            display_name: value.display_name,
            description: value.description,
            ..Default::default()
        };

        Ok(IoCattleManagementv3Cluster {
            metadata: Some(metadata),
            spec: Box::new(spec),
            ..Default::default()
        })
    }
}

impl PartialEq<Cluster> for IoCattleManagementv3Cluster {
    fn eq(&self, other: &Cluster) -> bool {
        let lhs = self.metadata.as_ref().and_then(|m| m.name.clone());
        let rhs = Some(other.id.clone());

        lhs == rhs
            && self.spec.display_name == other.display_name
            && self.spec.description == other.description
    }
}


impl PartialEq<IoCattleManagementv3Cluster> for Cluster {
    fn eq(&self, other: &IoCattleManagementv3Cluster) -> bool {
        other == self

        // let lhs = Some(self.id.clone());
        // let rhs = other.metadata.as_ref().and_then(|m| m.name.clone());

        // lhs == rhs
        //     && self.display_name == other.spec.display_name
        //     && self.description == other.spec.description
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cluster() -> Cluster {
        Cluster {
            id: "cluster-id".to_string(),
            display_name: "Test Cluster".to_string(),
            description: Some("A test cluster".to_string()),
        }
    }

    fn sample_iocattle_cluster() -> IoCattleManagementv3Cluster {
        IoCattleManagementv3Cluster {
            metadata: Some(IoK8sApimachineryPkgApisMetaV1ObjectMeta {
                name: Some("cluster-id".to_string()),
                ..Default::default()
            }),
            spec: Box::new(IoCattleManagementv3ClusterSpec {
                display_name: "Test Cluster".to_string(),
                description: Some("A test cluster".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_eq_cluster_and_iocattle_cluster() {
        let c = sample_cluster();
        let ioc = sample_iocattle_cluster();

        assert_eq!(c, ioc);
        assert_eq!(ioc, c);
    }

    #[test]
    fn test_inequality_on_display_name() {
        let mut ioc = sample_iocattle_cluster();
        ioc.spec.display_name = "Different Cluster".to_string();

        assert_ne!(sample_cluster(), ioc);
    }

    #[test]
    fn test_inequality_on_description() {
        let mut ioc = sample_iocattle_cluster();
        ioc.spec.description = Some("Different description".to_string());

        assert_ne!(sample_cluster(), ioc);
    }

    #[test]
    fn test_inequality_on_missing_metadata_name() {
        let mut ioc = sample_iocattle_cluster();
        ioc.metadata.as_mut().unwrap().name = None;

        assert_ne!(sample_cluster(), ioc);
    }

    #[test]
    fn test_try_from_iocattle_cluster_success() {
        let ioc = sample_iocattle_cluster();
        let cluster = Cluster::try_from(ioc).unwrap();

        assert_eq!(cluster.id, "cluster-id");
        assert_eq!(cluster.display_name, "Test Cluster");
        assert_eq!(cluster.description.as_deref(), Some("A test cluster"));
    }

    #[test]
    fn test_try_from_iocattle_cluster_missing_metadata() {
        let mut ioc = sample_iocattle_cluster();
        ioc.metadata = None;

        let result = Cluster::try_from(ioc);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "missing metadata");
    }

    #[test]
    fn test_try_from_iocattle_cluster_missing_name() {
        let mut ioc = sample_iocattle_cluster();
        ioc.metadata.as_mut().unwrap().name = None;

        let result = Cluster::try_from(ioc);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "missing name");
    }

    #[test]
    fn test_try_from_cluster_to_iocattle_cluster() {
        let cluster = sample_cluster();
        let ioc = IoCattleManagementv3Cluster::try_from(cluster.clone()).unwrap();

        assert_eq!(ioc.metadata.as_ref().unwrap().name.as_deref(), Some("cluster-id"));
        assert_eq!(ioc.spec.display_name, "Test Cluster");
        assert_eq!(ioc.spec.description.as_deref(), Some("A test cluster"));
    }


    #[test]
    fn test_eq_when_description_none_on_both() {
        let cluster = Cluster {
            description: None,
            ..sample_cluster()
        };

        let mut ioc = sample_iocattle_cluster();
        ioc.spec.description = None;

        assert_eq!(cluster, ioc);
        assert_eq!(ioc, cluster);
    }

    #[test]
    fn test_inequality_when_only_cluster_has_description() {
        let cluster = Cluster {
            description: Some("A test cluster".to_string()),
            ..sample_cluster()
        };

        let mut ioc = sample_iocattle_cluster();
        ioc.spec.description = None;

        assert_ne!(cluster, ioc);
        assert_ne!(ioc, cluster);
    }

    #[test]
    fn test_inequality_when_only_iocattle_has_description() {
        let cluster = Cluster {
            description: None,
            ..sample_cluster()
        };

        let mut ioc = sample_iocattle_cluster();
        ioc.spec.description = Some("A test cluster".to_string());

        assert_ne!(cluster, ioc);
        assert_ne!(ioc, cluster);
    }

    #[test]
    fn test_conversion_from_iocattle_cluster_with_none_description() {
        let mut ioc = sample_iocattle_cluster();
        ioc.spec.description = None;

        let result = Cluster::try_from(ioc).unwrap();
        assert_eq!(result.description, None);
    }

    #[test]
    fn test_conversion_to_iocattle_cluster_with_none_description() {
        let cluster = Cluster {
            description: None,
            ..sample_cluster()
        };

        let ioc = IoCattleManagementv3Cluster::try_from(cluster).unwrap();
        assert_eq!(ioc.spec.description, None);
    }




}
