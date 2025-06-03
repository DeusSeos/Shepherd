use std::fmt;
use std::env;
use std::{collections::HashMap, fmt::Display, path::PathBuf};

use rancher_client::models::{IoCattleManagementv3Cluster, IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding, IoCattleManagementv3RoleTemplate};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};

use crate::utils::git::GitAuth;
use crate::{cluster::Cluster, utils::file::FileFormat, resources::project::Project, resources::prtb::ProjectRoleTemplateBinding, resources::rt::RoleTemplate};

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
            writeln!(f, "  - {:?}", rt.display_name.as_ref().unwrap())?;
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


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ShepherdConfig {
    pub rancher_config_path: PathBuf,
    pub endpoint_url: String,
    pub file_format: FileFormat,
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_git_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_names: Option<Vec<String>>,
    #[serde(default = "default_loop_interval")]
    pub loop_interval: u64,
    #[serde(default = "default_retry_delay")]
    pub retry_delay: u64,
    pub auth_method: GitAuth,
    #[serde(default = "default_branch")]
    pub branch: String

}

impl ShepherdConfig {
    pub fn from_file(path: &str) -> Result<Self> {
        let file = std::fs::read_to_string(path).context("Failed to read config file")?;
        let mut config: ShepherdConfig = toml::from_str(&file).context("Failed to parse config file")?;

        // Handle Git authentication method
        config.auth_method = match (env::var("GIT_AUTH_METHOD"), env::var("GIT_SSH_KEY"), env::var("GIT_TOKEN")) {
            (Ok(method), Ok(key), _) if method == "ssh_key" => GitAuth::SshKey(PathBuf::from(key)),
            (Ok(method), _, Ok(token)) if method == "https_token" => GitAuth::HttpsToken(token),
            (Ok(method), _, _) if method == "ssh_agent" => GitAuth::SshAgent,
            (Ok(method), _, _) if method == "git_credential_helper" => GitAuth::GitCredentialHelper,
            _ => GitAuth::SshAgent, // default
        };
        Ok(config)
    }

    pub fn get_git_auth(&self) -> GitAuth {
        match (env::var("GIT_AUTH_METHOD"), env::var("GIT_SSH_KEY"), env::var("GIT_TOKEN")) {
            (Ok(method), Ok(key), _) if method == "ssh_key" => GitAuth::SshKey(PathBuf::from(key)),
            (Ok(method), _, Ok(token)) if method == "https_token" => GitAuth::HttpsToken(token),
            (Ok(method), _, _) if method == "ssh_agent" => GitAuth::SshAgent,
            (Ok(method), _, _) if method == "git_credential_helper" => GitAuth::GitCredentialHelper,
            _ => self.auth_method.clone(), // Use the value from the config file if environment variables are not set
        }
    }
}


fn default_loop_interval() -> u64 {
    300
}

fn default_retry_delay() -> u64 {
    200
}

fn default_branch() -> String {
    "main".to_string()
}


impl Display for ShepherdConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Rancher config path: {}", self.rancher_config_path.display())?;
        writeln!(f, "Endpoint URL: {}", self.endpoint_url)?;
        writeln!(f, "File format: {}", self.file_format)?;
        writeln!(
            f,
            "Remote git URL: {}",
            self.remote_git_url
                .as_deref()
                .unwrap_or("<none>")
        )?;
        writeln!(
            f,
            "Cluster names: {}",
            self.cluster_names
                .as_ref()
                .map(|v| v.join(", "))
                .unwrap_or_else(|| "<none>".into())
        )?;
        writeln!(f, "Loop interval: {} seconds", self.loop_interval)?;
        writeln!(f, "Retry delay: {} milliseconds", self.retry_delay)?;
        Ok(())
    }
}