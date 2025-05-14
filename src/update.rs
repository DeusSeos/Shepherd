use crate::diff::compute_cluster_diff;
use crate::project::update_project;
use crate::prtb::update_project_role_template_binding;
use crate::rt::update_role_template;
use crate::{load_configuration, load_configuration_from_rancher, ObjectType};
use crate::config::RancherClusterConfig;
use crate::file::FileFormat;
use serde_json::Value;
use std::path::Path;

pub async fn compare_and_update_configurations(
    configuration: &rancher_client::apis::configuration::Configuration,
    config_folder_path: &Path,
    cluster_id: &str,
    file_format: &FileFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the stored configuration
    let stored_config = load_configuration(config_folder_path, &configuration.base_path, cluster_id, file_format).await.unwrap().unwrap();
    let stored_config: RancherClusterConfig = RancherClusterConfig::try_from(stored_config).unwrap();

    // Load the live Rancher configuration
    let live_config = load_configuration_from_rancher(configuration, cluster_id).await;

    // Convert configurations to serde_json::Value for comparison
    let stored_config_value: Value = serde_json::to_value(stored_config)?;
    let live_config_value: Value = serde_json::to_value(live_config)?;

    // Compute the differences
    let diffs = compute_cluster_diff(&live_config_value, &stored_config_value);

    // Iterate through the differences and handle them
    // the key is ObjectType, object_id, namespace_id and value is the difference between the two states
    for ((object_type, object_id, namespace), diff_value) in diffs {
        // Add logic to handle the differences (e.g., update live configuration or log them)
        match object_type {
            ObjectType::Project => {
                let ns = namespace
                    .as_deref()
                    .unwrap_or("<no-namespace>");
                println!("  → project `{}` in namespace `{}`", object_id, ns);
                // Here you own `diff_value`, so you can consume it:
                update_project(configuration, &ns, &object_id, diff_value).await?;
            }
    
            ObjectType::RoleTemplate => {
                println!("  → role-template `{}`", object_id);
                update_role_template(configuration, &object_id, diff_value).await?;
            }
    
            ObjectType::ProjectRoleTemplateBinding => {
                let ns = namespace
                    .as_deref()
                    .unwrap_or("<no-namespace>");
                println!("  → prtb `{}` in namespace `{}`", object_id, ns);
                update_project_role_template_binding(configuration, &ns, &object_id, diff_value).await?;
            }
    
            _ => {
                // other cases, if any
                panic!("Unsupported object type: {:?}", object_type)
            }
        }
    }
    

    Ok(())
}