use rancher_client::models::{IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding, IoCattleManagementv3RoleTemplate};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ConversionError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid value for field '{field}': {reason}")]
    InvalidValue {
        field: &'static str,
        reason: String,
    },

    #[error("Failed to convert metadata: {0}")]
    MetadataError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}





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

/// The type of object to be updated in Rancher.
///
/// This enum represents the different types of objects that can be updated in Rancher. It includes:
/// - `Cluster`: Represents a cluster object.
/// - `Project`: Represents a project object.
/// - `RoleTemplate`: Represents a role template object.
/// - `ProjectRoleTemplateBinding`: Represents a project-role-template binding object.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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




