use std::collections::{BTreeSet, HashMap};

use json_patch::diff;
use rancher_client::models::{IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding, IoCattleManagementv3RoleTemplate};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tracing::{debug, info};

use crate::{clean_up_value, api::config::RancherClusterConfig, resources::project::PROJECT_EXCLUDE_PATHS, resources::prtb::PRTB_EXCLUDE_PATHS, resources::rt::RT_EXCLUDE_PATHS, models::ObjectType};


/// compute the cluster diff between the current state and the desired state
/// # Arguments
/// * `current_state` - The current state of the cluster
/// * `desired_state` - The desired state of the cluster
/// # Returns
/// * HashMap< (ObjectType, String, Option<String>), Value> - A HashMap containing the differences between the two states, key is the ObjectType, the String is the id of the object, and the Option<String> is the namespace of the object, and the Value is the difference between the two states
pub fn compute_cluster_diff(
    current_state: &Value,
    desired_state: &Value,
) -> HashMap< (ObjectType, String, Option<String>), Value> {

    let desired_state: RancherClusterConfig = serde_json::from_value(desired_state.clone()).unwrap();
    let current_state: RancherClusterConfig = serde_json::from_value(current_state.clone()).unwrap();

    let d_role_template = desired_state.role_templates.clone();
    let d_project = desired_state.projects.clone();

    let mut patches: HashMap<(ObjectType, String, Option<String>), Value> = HashMap::new();

    for drole_template in &d_role_template {
        if let Some(crt) = current_state
            .role_templates
            .iter()
            .find(|crole_template| crole_template.metadata.as_ref().unwrap().name == drole_template.metadata.as_ref().unwrap().name) {

            let mut drtv = serde_json::to_value(drole_template).unwrap();
            let mut crtv = serde_json::to_value(crt).unwrap();
            clean_up_value(&mut drtv, RT_EXCLUDE_PATHS);
            clean_up_value(&mut crtv, RT_EXCLUDE_PATHS);
            let patch = calculate_json_patch::<IoCattleManagementv3RoleTemplate>(&crtv, &drtv);
            let rt_id = drole_template.metadata.as_ref().unwrap().name.clone().unwrap();
            if let Some(patch) = patch {
                debug!("RoleTemplate `{}` diff computed and added to patches", rt_id);
                patches.insert((ObjectType::RoleTemplate, rt_id, None), patch);
            }
        }
    }

    for (d_project_id, (d_project, dprtbs)) in &d_project {
        if let Some((c_project, cprtbs)) = current_state.projects.get(d_project_id) {
            let mut dpv = serde_json::to_value(d_project).unwrap();
            let mut cpv = serde_json::to_value(c_project).unwrap();
            clean_up_value(&mut dpv, PROJECT_EXCLUDE_PATHS);
            clean_up_value(&mut cpv, PROJECT_EXCLUDE_PATHS);
            let patch = calculate_json_patch::<IoCattleManagementv3Project>(&cpv, &dpv);
            let cluster_id = d_project.metadata.as_ref().unwrap().namespace.clone().unwrap();
            if let Some(patch) = patch {
                patches.insert((ObjectType::Project, d_project_id.to_string(), Some(cluster_id.clone())), patch);
                debug!("Project `{}` diff computed and added to patches", d_project_id);
            }

            for dprtb in dprtbs {
                if let Some(cprtb) = cprtbs.iter().find(|cprtb| cprtb.metadata.as_ref().unwrap().name == dprtb.metadata.as_ref().unwrap().name) {
                    let mut dprtbv = serde_json::to_value(dprtb).unwrap();
                    let mut cprtbv = serde_json::to_value(cprtb).unwrap();
                    clean_up_value(&mut dprtbv, PRTB_EXCLUDE_PATHS);
                    clean_up_value(&mut cprtbv, PRTB_EXCLUDE_PATHS);
                    let patch = calculate_json_patch::<IoCattleManagementv3ProjectRoleTemplateBinding>(&cprtbv, &dprtbv);
                    let prtb_id = dprtb.metadata.as_ref().unwrap().name.clone().unwrap();
                    if let Some(patch) = patch {
                        debug!("ProjectRoleTemplateBinding `{}` diff computed and added to patches", prtb_id);
                        patches.insert((ObjectType::ProjectRoleTemplateBinding, prtb_id, Some(d_project_id.clone())), patch);
                    }
                }
            }
        } else {
            debug!("Project `{}` not found in current state", d_project_id);
            // Missing project so create it
            
        }
    }
    info!("Total patches computed: {}", patches.len());
    patches
}


/// Compare two optional annotation‐maps and print per‐key changes.
/// # Arguments
/// * `a` - The first optional annotation‐map.
/// * `b` - The second optional annotation‐map.
///
pub fn diff_boxed_hashmap_string_string(
    a: Option<&HashMap<String, String>>,
    b: Option<&HashMap<String, String>>,
) {
    debug!("diff_boxed_hashmap_string_string: a={:#?} b={:#?}", a, b);

    // // Treat None as empty map
    // let binding = HashMap::new();
    // let ma = a.unwrap_or(binding);
    // let binding = HashMap::new();
    // let mb = b.unwrap_or(binding);

    let ma = a.as_ref().unwrap();
    let mb = b.as_ref().unwrap();

    debug!("diff_boxed_hashmap_string_string: ma={:#?} mb={:#?}", ma, mb);

    // Collect all keys
    let keys: BTreeSet<_> = ma.keys().chain(mb.keys()).collect();

    debug!("diff_boxed_hashmap_string_string: keys={:#?}", &keys);

    for key in keys {
        debug!("diff_boxed_hashmap_string_string: Comparing key {}", key);

        match (ma.get(key), mb.get(key)) {
            (Some(old), Some(new)) if old != new => {
                println!("Hashmap changed  {}: {:?} → {:?}", key, old, new);
            }
            (None, Some(new)) => {
                println!("Hashmap added    {}: {:?}", key, new);
            }
            (Some(old), None) => {
                println!("Hashmap removed  {}: {:?}", key, old);
            }
            _ => { /* unchanged */ }
        }
    }
}


/// Create a JSON patch between two JSON values.
/// # Arguments
/// * `current_state` - The current state of the JSON object.
/// * `desired_state` - The desired state of the JSON object.
/// # Returns
/// * A JSON value representing the patch.
///
pub fn calculate_json_patch<T>(current_state: &Value, desired_state: &Value) -> Option<Value>
where
    T: Serialize + DeserializeOwned,
{
    // enforce conversion to IoCattleManagementv3Project
    let current: T = serde_json::from_value(current_state.clone()).unwrap();
    let desired: T = serde_json::from_value(desired_state.clone()).unwrap();

    // Serialize back to JSON values
    let current_value = serde_json::to_value(current).unwrap();
    let desired_value = serde_json::to_value(desired).unwrap();

    // Compute the JSON patch
    let patch = diff(&current_value, &desired_value);

    // Convert the patch to a JSON value if it isn't empty
    if !patch.is_empty() {
        return Some(serde_json::to_value(patch).unwrap())
    }
    None
}