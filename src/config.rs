use std::{collections::HashMap, fmt::Display};

use rancher_client::models::{IoCattleManagementv3Cluster, IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding, IoCattleManagementv3RoleTemplate};
use serde::{Deserialize, Serialize};

use crate::{cluster::Cluster, project::Project, prtb::ProjectRoleTemplateBinding, rt::RoleTemplate};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ClusterConfig {
    pub cluster: Cluster,
    pub role_templates: Vec<RoleTemplate>,
    /// Map from project ID → (project, its role‐template‐bindings)
    pub projects: HashMap<String, (Project, Vec<ProjectRoleTemplateBinding>)>,
}

impl Display for ClusterConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cluster: {}", self.cluster.display_name)?;
        writeln!(f, "Role Templates:")?;
        for rt in &self.role_templates {
            writeln!(f, "  - {:?}", rt.display_name)?;
        }
        writeln!(f, "Projects:")?;
        for (project_id, (project, bindings)) in &self.projects {
            writeln!(f, "  - {} (ID: {})", project.display_name, project_id)?;
            for binding in bindings {
                writeln!(f, "    - Binding: {}", binding.id)?;
            }
        }
        Ok(())
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RancherClusterConfig {
    pub cluster: IoCattleManagementv3Cluster,
    pub role_templates: Vec<IoCattleManagementv3RoleTemplate>,
    /// Map from project ID → (project, its role‐template‐bindings)
    pub projects: HashMap<String, (IoCattleManagementv3Project, Vec<IoCattleManagementv3ProjectRoleTemplateBinding>)>,
}

