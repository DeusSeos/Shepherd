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


// conversion from ClusterConfig to RancherClusterConfig

impl TryFrom<ClusterConfig> for RancherClusterConfig {
    type Error = &'static str;

    fn try_from(value: ClusterConfig) -> Result<Self, Self::Error> {
        // 1. Cluster
        let rancher_cluster =
            IoCattleManagementv3Cluster::try_from(value.cluster).map_err(|_| "cluster conversion failed")?;

        // 2. Role-templates: map + collect into Vec<…>
        let rancher_role_templates = value
            .role_templates
            .into_iter()
            .map(|rt| {
                IoCattleManagementv3RoleTemplate::try_from(rt)
                    .map_err(|_| "role-template conversion failed")
            })
            .collect::<Result<Vec<_>, _>>()?;

        // 3. Projects → HashMap<String, (Project, Vec<Binding>)>
        let rancher_projects = value
        .projects
        .into_iter()
        .map(|(project_id, (project, bindings))| -> Result<
            (String, (IoCattleManagementv3Project, Vec<IoCattleManagementv3ProjectRoleTemplateBinding>)),
            &'static str,
        > {
            // convert the Project
            let rp = IoCattleManagementv3Project::try_from(project)
                .map_err(|_| "project conversion failed")?;
            // convert each binding
            let rb = bindings
                .into_iter()
                .map(|b| {
                    IoCattleManagementv3ProjectRoleTemplateBinding::try_from(b)
                        .map_err(|_| "binding conversion failed")
                })
                .collect::<Result<Vec<_>, &'static str>>()?;
            // here you return the one `(key, value)` pair
            Ok((project_id, (rp, rb)))
        })
        // now collect into a HashMap<_, _>, propagating any of the &'static str errors
        .collect::<Result<
            HashMap<String, (IoCattleManagementv3Project, Vec<IoCattleManagementv3ProjectRoleTemplateBinding>)>,
            &'static str,
        >>()?;


        Ok(RancherClusterConfig {
            cluster: rancher_cluster,
            role_templates: rancher_role_templates,
            projects: rancher_projects,
        })
    }
}