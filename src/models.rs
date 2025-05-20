use std::borrow::Cow;

use rancher_client::models::{IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding, IoCattleManagementv3RoleTemplate};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{project::Project, prtb::ProjectRoleTemplateBinding, rt::RoleTemplate};

#[derive(Debug, Error, PartialEq, Clone)]
pub enum ConversionError {
    #[error("Missing required field: {0}")]
    MissingField(Cow<'static, str>),

    #[error("Invalid value for field '{field}': {reason}")]
    InvalidValue {
        field: Cow<'static, str>,
        reason: Cow<'static, str>,
    },

    #[error("Failed to convert metadata: {0}")]
    MetadataError(Cow<'static, str>),

    #[error("Other conversion error: {0}")]
    Other(Cow<'static, str>),
}

impl From<serde_json::Error> for ConversionError {
    fn from(err: serde_json::Error) -> Self {
        ConversionError::Other(err.to_string().into())
    }
}

impl From<serde_yaml::Error> for ConversionError {
    fn from(err: serde_yaml::Error) -> Self {
        ConversionError::Other(err.to_string().into())
    }
}

impl From<toml::de::Error> for ConversionError {
    fn from(err: toml::de::Error) -> Self {
        ConversionError::Other(err.to_string().into())
    }
}

// Implement Into for ConversionError
impl Into<ConversionError> for Box<dyn std::error::Error + Send + Sync> {
    fn into(self) -> ConversionError {
        ConversionError::Other(self.to_string().into())
    }
}


impl Into<ConversionError> for std::io::Error {
    fn into(self) -> ConversionError {
        ConversionError::Other(self.to_string().into())
    }
}






#[derive(Debug, Clone, Copy)]
pub enum ResourceVersionMatch {
    Exact,
    NotOlderThan,
}

impl ResourceVersionMatch {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceVersionMatch::Exact => "Exact",
            ResourceVersionMatch::NotOlderThan => "notOlderThan",
        }
    }
}
impl std::fmt::Display for ResourceVersionMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
impl std::str::FromStr for ResourceVersionMatch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "exact" => Ok(ResourceVersionMatch::Exact),
            "notolderthan" => Ok(ResourceVersionMatch::NotOlderThan),
            _ => Err(()),
        }
    }
}



// TryFrom for Project
#[derive(Debug, Clone)]
pub struct MinimalObject {
    pub object_id: Option<String>,
    pub resource_version_match: ResourceVersionMatch,
    pub resource_version: Option<String>,
    pub namespace: Option<String>,
}

// TryFrom for &Project
impl TryFrom<&Project> for MinimalObject {
    type Error = ConversionError;

    fn try_from(value: &Project) -> Result<Self, Self::Error> {
        Ok(MinimalObject {
            object_id: value.id.clone(),
            resource_version_match: ResourceVersionMatch::Exact,
            resource_version: value.resource_version.clone(),
            namespace: Some(value.namespace.clone()),
        })
    }
}

// TryFrom for Project
impl TryFrom<Project> for MinimalObject {
    type Error = ConversionError;

    fn try_from(value: Project) -> Result<Self, Self::Error> {
        Ok(MinimalObject {
            object_id: value.id,
            resource_version_match: ResourceVersionMatch::Exact,
            resource_version: value.resource_version,
            namespace: Some(value.namespace),
        })
    }
}

// TryFrom for &ProjectRoleTemplateBinding
impl TryFrom<&ProjectRoleTemplateBinding> for MinimalObject {
    type Error = ConversionError;

    fn try_from(value: &ProjectRoleTemplateBinding) -> Result<Self, Self::Error> {
        Ok(MinimalObject {
            object_id: Some(value.id.clone()),
            resource_version_match: ResourceVersionMatch::Exact,
            resource_version: value.resource_version.clone(),
            namespace: Some(value.namespace.clone()),
        })
    }
}

// TryFrom for ProjectRoleTemplateBinding
impl TryFrom<ProjectRoleTemplateBinding> for MinimalObject {
    type Error = ConversionError;

    fn try_from(value: ProjectRoleTemplateBinding) -> Result<Self, Self::Error> {
        Ok(MinimalObject {
            object_id: Some(value.id),
            resource_version_match: ResourceVersionMatch::Exact,
            resource_version: value.resource_version,
            namespace: Some(value.namespace),
        })
    }
}

// TryFrom for &RoleTemplate
impl TryFrom<&RoleTemplate> for MinimalObject {
    type Error = ConversionError;

    fn try_from(value: &RoleTemplate) -> Result<Self, Self::Error> {
        Ok(MinimalObject {
            object_id: Some(value.id.clone()),
            resource_version_match: ResourceVersionMatch::Exact,
            resource_version: value.resource_version.clone(),
            namespace: None, // Assuming RoleTemplate doesn't have a namespace
        })
    }
}

// TryFrom for RoleTemplate
impl TryFrom<RoleTemplate> for MinimalObject {
    type Error = ConversionError;

    fn try_from(value: RoleTemplate) -> Result<Self, Self::Error> {
        Ok(MinimalObject {
            object_id: Some(value.id),
            resource_version_match: ResourceVersionMatch::Exact,
            resource_version: value.resource_version,
            namespace: None, // Assuming RoleTemplate doesn't have a namespace
        })
    }
}



/// The type of object to be updated in Rancher.
///
/// This enum represents the different types of objects that can be updated in Rancher. It includes:
/// - `Cluster`: Represents a cluster object.
/// - `Project`: Represents a project object.
/// - `RoleTemplate`: Represents a role template object.
/// - `ProjectRoleTemplateBinding`: Represents a project-role-template binding object.
///
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    RoleTemplate,
    Project,
    ProjectRoleTemplateBinding,
    Cluster,
}




pub enum CreatedObject {
    // Cluster(Cluster),
    Project(IoCattleManagementv3Project),
    RoleTemplate(IoCattleManagementv3RoleTemplate),
    ProjectRoleTemplateBinding(IoCattleManagementv3ProjectRoleTemplateBinding),
}




