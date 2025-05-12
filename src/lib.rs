// This file will contain all the functions that will be used to interact and extract from the Rancher API
pub mod cluster;
pub mod config;
pub mod git;
pub mod project;
pub mod prtb;
pub mod rt;

use json_patch::diff;
use serde_json::Value;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use std::option::Option;
use std::collections::{BTreeSet, HashMap};

use cluster::Cluster;
use config::{ClusterConfig, RancherClusterConfig};
use project::{Project, PROJECT_EXCLUDE_PATHS};
use prtb::{ProjectRoleTemplateBinding, PRTB_EXCLUDE_PATHS};
use rt::{get_role_templates, RoleTemplate, RT_EXCLUDE_PATHS};

use rancher_client::models::{IoCattleManagementv3Cluster, IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding, IoCattleManagementv3RoleTemplate};
use rancher_client::apis::configuration::{ApiKey, Configuration};


pub fn rancher_config_init(host: &str, token: &str) -> Configuration {
    let mut config = Configuration::new();
    config.base_path = host.to_string();

    config.api_key = Some(ApiKey {
        prefix: Some("Bearer".to_string()),
        key: token.to_string(),
    });
    config
}

// Local usage only, for testing purposes
// This function will be used to fetch the current configuration from the Rancher API
// it will do the following:
// 1. Get the current configuration from the Rancher API for clusters, projects, role templates and project role template bindings
// 2. Loop through the clusters and save them to a folder
// 3. Loop through the projects and save them to the correct cluster folder
// 4. Loop through the role templates and save them to the correct cluster folder
// 5. Loop through the project role template bindings and save them to the correct project folder

//
// the example folder structure will be as follows:
// /c-293x
// ├─ c-293x.(yaml/json/toml)
// ├─ /p-2a4i21
// │　├─ p-2a4i21.(yaml/json/toml)
// │　└─ prtb-29291.(yaml/json/toml)
// ├─ /p-8a2h12
// │　├─ p-8a2h12.(yaml/json/toml)
// │　└─ prtb-9nn91.(yaml/json/toml)
// └─ roles
// 　　├─ rt-92813.(yaml/json/toml)
// 　　└─ rt-92818.(yaml/json/toml)

// the file names will be the ID of the object
// the file extension will be the format of the file
// the folder names will be the ID of the object
// the folder will be created if it does not exist
// the function will return the path to the folderm

pub async fn download_current_configuration(
    configuration: &Configuration,
    path: &Path,
    file_format: &FileFormat,
) {
    // Get the current configuration from the Rancher API
    let rancher_cluster = cluster::get_clusters(configuration)
        .await
        .map_err(|e| {
            println!("Failed to get clusters: {:?}", e);
            std::process::exit(1);
        })
        .unwrap();
    let rancher_role_templates =
        get_role_templates(configuration, None, None, None, None, None, None)
            .await
            .map_err(|e| {
                println!("Failed to get role templates: {:?}", e);
                std::process::exit(1);
            })
            .unwrap();

    // Create the base folder if it does not exist. Base folder will be the path provided and the endpoint url
    // for example: /tmp/rancher_config/https://rancher.rd.localhost

    let base_path = path.join(
        configuration
            .base_path
            .replace("https://", "")
            .replace("/", "_"),
    );
    if !base_path.exists() {
        let _ = std::fs::create_dir_all(&base_path).map_err(|e| {
            println!("Failed to create folder: {:?}", e);
            std::process::exit(1);
        });
    }
    // create the folder for the role templates if it does not exist
    let role_template_path = base_path.join("roles");
    if !role_template_path.exists() {
        let _ = std::fs::create_dir_all(&role_template_path).map_err(|e| {
            println!("Failed to create folder: {:?}", e);
            std::process::exit(1);
        });
    }

    // convert the role templates to our simple struct for each object using the try_from method
    let role_templates: Vec<RoleTemplate> = rancher_role_templates
        .items
        .into_iter()
        .map(|role_template| {
            // try to convert the IoCattleManagementV3RoleTemplate to our simple RoleTemplate struct
            // if it fails, return an error
            role_template
                .try_into()
                .map_err(|e| {
                    println!("Failed to convert role template: {:?}", e);
                    std::process::exit(1);
                })
                .unwrap()
        })
        .collect::<Vec<RoleTemplate>>();

    // Loop through the role templates and save them to the folder
    for role_template in &role_templates {
        // save the role template to the folder
        let role_template_file = role_template_path.join(format!(
            "{}.{}",
            role_template.id,
            file_extension_from_format(file_format)
        ));
        let _ = std::fs::write(
            &role_template_file,
            serialize_object(role_template, file_format),
        )
        .map_err(|e| {
            println!("Failed to write file: {:?}", e);
            std::process::exit(1);
        });
    }

    // convert the items to our simple struct for each object using the try_from method
    let clusters: Vec<Cluster> = rancher_cluster
        .items
        .into_iter()
        .map(|cluster| {
            // try to convert the IoCattleManagementV3Cluster to our simple Cluster struct
            // if it fails, return an error
            cluster
                .try_into()
                .map_err(|e| {
                    println!("Failed to convert cluster: {:?}", e);
                    std::process::exit(1);
                })
                .unwrap()
        })
        .collect::<Vec<Cluster>>();

    // Loop through the clusters and save them to the folder
    for cluster in &clusters {
        // create the folder for the cluster if it does not exist
        let cluster_path = base_path.join(cluster.id.clone());
        if !cluster_path.exists() {
            let _ = std::fs::create_dir_all(&cluster_path).map_err(|e| {
                println!("Failed to create folder: {:?}", e);
                std::process::exit(1);
            });
        }

        // save the cluster to the folder
        let cluster_file = cluster_path.join(format!(
            "{}.{}",
            cluster.id,
            file_extension_from_format(file_format)
        ));
        let _ =
            std::fs::write(&cluster_file, serialize_object(cluster, file_format)).map_err(|e| {
                println!("Failed to write file: {:?}", e);
                std::process::exit(1);
            });

        // fetch the projects for the cluster
        let rancher_projects = project::get_projects(
            configuration,
            &cluster.id,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| {
            println!("Failed to get projects: {:?}", e);
            std::process::exit(1);
        })
        .unwrap();
        // convert the items to our simple struct for each object using the try_from method
        let projects: Vec<Project> = rancher_projects
            .items
            .into_iter()
            .map(|project| {
                // try to convert the IoCattleManagementV3Project to our simple Project struct
                // if it fails, return an error
                project
                    .try_into()
                    .map_err(|e| {
                        println!("Failed to convert project: {:?}", e);
                        std::process::exit(1);
                    })
                    .unwrap()
            })
            .collect::<Vec<project::Project>>();

        // Loop through the projects and save them to the folder
        for project in &projects {
            // create the folder for the project if it does not exist
            let project_path = cluster_path.join(project.id.clone());
            if !project_path.exists() {
                let _ = std::fs::create_dir_all(&project_path).map_err(|e| {
                    println!("Failed to create folder: {:?}", e);
                    std::process::exit(1);
                });
            }

            // save the project to the folder
            let project_file = project_path.join(format!(
                "{}.{}",
                project.id,
                file_extension_from_format(file_format)
            ));
            let _ = std::fs::write(&project_file, serialize_object(project, file_format)).map_err(
                |e| {
                    println!("Failed to write file: {:?}", e);
                    std::process::exit(1);
                },
            );

            let rancher_project_role_template_bindings =
                prtb::get_namespaced_project_role_template_bindings(
                    configuration,
                    &project.id,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(|e| {
                    println!("Failed to get project role template bindings: {:?}", e);
                    std::process::exit(1);
                })
                .unwrap();

            // TODO: convert the conversion to a generic function
            let prtbs: Vec<ProjectRoleTemplateBinding> = rancher_project_role_template_bindings
                .items
                .into_iter()
                .map(|prtb| {
                    // try to convert the IoCattleManagementV3ProjectRoleTemplateBinding to our simple ProjectRoleTemplateBinding struct
                    // if it fails, return an error
                    prtb.try_into()
                        .map_err(|e| {
                            println!("Failed to convert project role template binding: {:?}", e);
                            std::process::exit(1);
                        })
                        .unwrap()
                })
                .collect::<Vec<prtb::ProjectRoleTemplateBinding>>();

            // Loop through the project role template bindings and save them to the folder
            for prtb in &prtbs {
                // save the project role template binding to the project folder
                let prtb_file = project_path.join(format!(
                    "{}.{}",
                    prtb.id,
                    file_extension_from_format(file_format)
                ));
                let _ =
                    std::fs::write(&prtb_file, serialize_object(prtb, file_format)).map_err(|e| {
                        println!("Failed to write file: {:?}", e);
                        std::process::exit(1);
                    });
            }
        }
    }
}

pub async fn load_configuration_from_rancher(
    configuration: &Configuration,
    cluster_id: &str) -> RancherClusterConfig {

    

    // Get the current configuration from the Rancher API
    let rancher_clusters = cluster::get_clusters(configuration)
        .await
        .map_err(|e| {
            println!("Failed to get clusters: {:?}", e);
            std::process::exit(1);
        })
        .unwrap();
    let rancher_cluster = rancher_clusters
        .items
        .into_iter()
        .find(|cluster| cluster.metadata.as_ref().and_then(|m| m.name.as_deref()) == Some(cluster_id))
        .unwrap();

    let rancher_role_templates = get_role_templates(
        configuration,
        None,
        None,
        None,
        None,
        None,
        None,
    ).await.map_err(|e| {
        println!("Failed to get role templates: {:?}", e);
        std::process::exit(1);
    }).unwrap();

    let rrt: Vec<IoCattleManagementv3RoleTemplate> = rancher_role_templates.items.clone();

    // let cluster_projects: HashMap<String, (IoCattleManagementv3Project, Vec<IoCattleManagementv3ProjectRoleTemplateBinding>)> = HashMap::new();

    let rancher_projects = project::get_projects(
        configuration,
        cluster_id,
        None,
        None,
        None,
        None,
        None,
        None,
    ).await.map_err(|e| {
        println!("Failed to get projects: {:?}", e);
        std::process::exit(1);
    }).unwrap();


    let mut rancher_cluster_config = RancherClusterConfig {
        cluster: rancher_cluster,
        role_templates: rrt,
        projects: HashMap::new(),
    };

    let rprojects: Vec<IoCattleManagementv3Project> = rancher_projects.items.clone();
        
    for rproject in rprojects {
        let project = rproject.clone();
        let project_id = project.metadata.as_ref().and_then(|m| m.name.as_deref()).unwrap();
        let rancher_project_role_template_bindings =
            prtb::get_namespaced_project_role_template_bindings(
                configuration,
                &project_id,
                None,
                None,
                None,
                None,
                None,
                None,
            ).await.map_err(|e| {
            println!("Failed to get project role template bindings: {:?}", e);
            std::process::exit(1);
        }).unwrap();
        let rprtbs: Vec<IoCattleManagementv3ProjectRoleTemplateBinding> = rancher_project_role_template_bindings.items.clone();
        
        rancher_cluster_config.projects.insert(
            project_id.to_string(),
            (project, rprtbs),
        );
    }


    return rancher_cluster_config;

    }





// load the entire configuration from the base path
///
/// # Arguments
/// `path`: The path for the directory to load the configuration from
/// `endpoint_url`: The endpoint URL to load the configuration from (relates to the directory name where the whole cluster configuration is stored)
/// `cluster_id`: The cluster ID to load the configuration from
/// `file_format`: The file format to load the configuration from
/// # Returns
/// `Option<ClusterConfig>`: The configuration object
pub async fn load_configuration(
    path: &Path,
    endpoint_url: &str,
    cluster_id: &str,
    file_format: &FileFormat,
) -> Result<Option<ClusterConfig>, ()> {
    // create the path to the endpoint
    let endpoint_path = path.join(endpoint_url.replace("https://", "").replace("/", "_"));
    // check if the path exists
    if !endpoint_path.exists() {
        println!("Configuration path does not exist");
        return Err(());
    }
    // create the cluster path
    let cluster_folder_path = endpoint_path.join(cluster_id);
    // check if the path exists
    if !cluster_folder_path.exists() {
        println!("Cluster path does not exist");
        return Err(());
    }

    // read the cluster file
    let cluster_file = cluster_folder_path.join(format!(
        "{}.{}",
        cluster_id,
        file_extension_from_format(file_format)
    ));
    // check if the file exists
    if !cluster_file.exists() {
        println!("Cluster file does not exist");
        return Err(());
    }

    // read the file and deserialize it
    let cluster_file_content = std::fs::read_to_string(&cluster_file)
        .map_err(|e| {
            println!("Failed to read file: {:?}", e);
            e
        })
        .unwrap();

    let cluster: Cluster = deserialize_object(&cluster_file_content, file_format);

    let mut cluster_config = ClusterConfig {
        cluster: cluster.clone(),
        role_templates: Vec::new(),
        projects: HashMap::new(),
    };

    // read the role templates
    let role_template_path = endpoint_path.join("roles");
    println!("Role template path: {:?}", role_template_path);

    // check if the path exists
    if !role_template_path.exists() {
        println!("Role template path does not exist");
        std::process::exit(1);
    }
    // read the role template files
    let role_template_files = std::fs::read_dir(&role_template_path)
        .map_err(|e| {
            println!("Failed to read directory: {:?}", e);
            e
        })
        .unwrap();
    // create a vector to hold the role templates
    let mut role_templates: Vec<RoleTemplate> = Vec::new();
    // loop through the files and deserialize them
    for role_template_file in role_template_files {
        // get the file name
        let role_template_file = role_template_file.unwrap();
        // check if the file is a file
        if role_template_file.file_type().unwrap().is_file() {
            // read the file and deserialize it
            let role_template_file_content = std::fs::read_to_string(role_template_file.path())
                .map_err(|e| {
                    println!("Failed to read file: {:?}", e);
                    std::process::exit(1);
                })
                .unwrap();
            // deserialize the file
            let role_template: RoleTemplate =
                deserialize_object(&role_template_file_content, file_format);
            // add the role template to the vector
            role_templates.push(role_template);
        }
    }
    // add the role templates to the cluster config
    cluster_config.role_templates = role_templates.clone();

    // read the project folders
    let project_folders = std::fs::read_dir(&cluster_folder_path)
        .map_err(|e| {
            println!("Failed to read directory: {:?}", e);
            e
        })
        .unwrap();

    // loop through the folders and deserialize them
    for project_folder in project_folders {
        // get the folder name
        let project_folder = project_folder.unwrap();
        // check if the folder is a directory
        if project_folder.file_type().unwrap().is_dir() {
            // read the project file
            let project_file = project_folder.path().join(format!(
                "{}.{}",
                project_folder.file_name().to_str().unwrap(),
                file_extension_from_format(file_format)
            ));
            // check if the file exists
            if !project_file.exists() {
                println!("Project file does not exist");
                return Err(());
            }
            // read the file and deserialize it
            let project_file_content = std::fs::read_to_string(&project_file)
                .map_err(|e| {
                    println!("Failed to read file: {:?}", e);
                    e
                })
                .unwrap();
            let project: Project = deserialize_object(&project_file_content, file_format);

            // add the project to the cluster config
            cluster_config
                .projects
                .insert(project.id.clone(), (project.clone(), Vec::new()));

            // read the project role template binding files, exclude the project file
            let prtb_files = std::fs::read_dir(project_folder.path())
                .map_err(|e| {
                    println!("Failed to read directory: {:?}", e);
                    e
                })
                .unwrap();
            // exclude the project file from the list of files
            let prtb_files: Vec<_> = prtb_files
                .filter(|prtb_file| {
                    // check if the file is a file
                    let prtb_file = prtb_file.as_ref().unwrap();
                    if prtb_file.file_type().unwrap().is_file() {
                        // check if the file is not the project file
                        if prtb_file.path() != project_file {
                            return true;
                        }
                    }
                    false
                })
                .collect();
            // create a vector to hold the project role template bindings
            let mut project_role_template_bindings: Vec<ProjectRoleTemplateBinding> = Vec::new();
            // loop through the files and deserialize them
            for prtb_file in prtb_files {
                // get the file name
                let prtb_file = prtb_file.unwrap();
                // check if the file is a file
                if prtb_file.file_type().unwrap().is_file() {
                    // read the file and deserialize it
                    let prtb_file_content = std::fs::read_to_string(prtb_file.path())
                        .map_err(|e| {
                            println!("Failed to read file: {:?}", e);
                            std::process::exit(1);
                        })
                        .unwrap();
                    // deserialize the file
                    let prtb: ProjectRoleTemplateBinding =
                        deserialize_object(&prtb_file_content, file_format);
                    // add the project role template binding to the vector
                    project_role_template_bindings.push(prtb);
                }
            }
            // add the project role template bindings to the cluster config
            // check if the project exists in the cluster config
            if let Some((_, prtbs)) = cluster_config.projects.get_mut(&project.id) {
                // add the project role template bindings to the project
                prtbs.extend(project_role_template_bindings);
            } else {
                println!("Project not found in cluster config");
                return Err(());
            }
        }
    }

    Ok(Some(cluster_config))
}

/// compute the cluster diff between the current state and the desired state
/// # Arguments
/// * `current_state` - The current state of the cluster
/// * `desired_state` - The desired state of the cluster
/// # Returns
/// * Vec<Value>: The diffs between the current state and the desired state
pub fn compute_cluster_diff(
    current_state: &Value,
    desired_state: &Value,
) -> Vec<Value> {

    // create a new rancher cluster object
    // convert to RancherClusterConfig
    let current_state: RancherClusterConfig = serde_json::from_value(current_state.clone()).unwrap();
    let desired_state: RancherClusterConfig = serde_json::from_value(desired_state.clone()).unwrap();


    // let cluster = current_state.cluster.clone();
    // create a new role template object
    let c_role_template = current_state.role_templates.clone();
    // create a new project object
    let c_project = current_state.projects.clone();

    let mut patches = Vec::new();

    // loop through the role templates and compare them
    for crt in &c_role_template {
        // check if the role template exists in the desired state
        if let Some(desired_rt) = desired_state
            .role_templates
            .iter()
            .find(|drole_template| drole_template.metadata.as_ref().unwrap().name == crt.metadata.as_ref().unwrap().name) {
            // compute the diff between the current state and the desired state
            // convert the current state to a JSON value
            let mut crtv = serde_json::to_value(crt).unwrap();
            let mut drtv = serde_json::to_value(desired_rt).unwrap();
            clean_up_value(&mut crtv, RT_EXCLUDE_PATHS);
            clean_up_value(&mut drtv, RT_EXCLUDE_PATHS);
            let patch = create_json_patch::<RoleTemplate>(&crtv, &drtv);
            patches.push(patch);
        }
    }

    // loop through the projects and compare them
    for (c_project_id, (c_project, cprtbs)) in &c_project {
        // check if the project exists in the desired state
        if let Some((d_project, dprtbs)) = desired_state.projects.get(c_project_id) {

            let mut cpv = serde_json::to_value(c_project).unwrap();
            let mut dpv = serde_json::to_value(d_project).unwrap();
            clean_up_value(&mut cpv, PROJECT_EXCLUDE_PATHS);
            clean_up_value(&mut dpv, PROJECT_EXCLUDE_PATHS);
            let patch = create_json_patch::<Project>(&cpv, &dpv);
            patches.push(patch);

            // loop through the project role template bindings and compare them
            for cprtb in cprtbs {
                // check if the project role template binding exists in the desired state
                if let Some(desired_prtb) = dprtbs.iter().find(|dprtb| dprtb.metadata.as_ref().unwrap().name == cprtb.metadata.as_ref().unwrap().name) {
                    let mut cprtbv = serde_json::to_value(cprtb).unwrap();
                    let mut dprtbv = serde_json::to_value(desired_prtb).unwrap();
                    clean_up_value(&mut cprtbv, PRTB_EXCLUDE_PATHS);
                    clean_up_value(&mut dprtbv, PRTB_EXCLUDE_PATHS);
                    let patch = create_json_patch::<ProjectRoleTemplateBinding>(&cprtbv, &dprtbv);
                    patches.push(patch);
                }
            }
        }
    }

    patches
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
pub async fn load_project(
    base_path: &Path,
    endpoint_url: &str,
    cluster_id: &str,
    project_id: &str,
    file_format: FileFormat,
) -> Project {
    // create the path to the project
    let project_path = base_path
        .join(endpoint_url.replace("https://", "").replace("/", "_"))
        .join(cluster_id)
        .join(project_id);
    // check if the path exists
    if !project_path.exists() {
        println!("Project path does not exist");
        std::process::exit(1);
    }
    // read the file from the path
    let project_file = project_path.join(format!(
        "{}.{}",
        project_id,
        file_extension_from_format(&file_format)
    ));
    // check if the file exists
    if !project_file.exists() {
        println!("Project file does not exist");
        std::process::exit(1);
    }
    // read the file and deserialize it
    let project_file_content = std::fs::read_to_string(&project_file)
        .map_err(|e| {
            println!("Failed to read file: {:?}", e);
            std::process::exit(1);
        })
        .unwrap();

    deserialize_object(&project_file_content, &file_format)
}

/// Recursively remove fields from a JSON Value based on a list of dot-separated paths.
/// # Arguments
/// * `value` - The mutable JSON object to clean
/// * `exclude_paths` - A list of dot-separated paths to remove (e.g., ["status", "metadata.creationTimestamp"])
pub fn clean_up_value(value: &mut Value, exclude_paths: &[&str]) {
    for path in exclude_paths {
        let parts: Vec<&str> = path.split('.').collect();
        remove_path_and_return(value, &parts);
    }
}

/// Remove a deeply nested field from a JSON object and remove it.
/// Traverses objects by key. Returns `None` if any key is missing or the path is invalid.
fn remove_path_and_return(value: &mut Value, path: &[&str]) -> Option<Value> {
    if path.is_empty() {
        return None;
    }

    let mut current = value;

    // Traverse to the parent of the key to remove
    for &key in &path[..path.len() - 1] {
        current = current.as_object_mut()?.get_mut(key)?;
    }

    let last_key = *path.last().unwrap();

    // Remove the target key
    current.as_object_mut().unwrap().remove(last_key)
}

/// Compare two optional annotation‐maps and print per‐key changes.
/// # Arguments
/// * `a` - The first optional annotation‐map.
/// * `b` - The second optional annotation‐map.
///
fn diff_boxed_hashmap_string_string(
    a: Option<&HashMap<String, String>>,
    b: Option<&HashMap<String, String>>,
) {
    // // Treat None as empty map
    // let binding = HashMap::new();
    // let ma = a.unwrap_or(binding);
    // let binding = HashMap::new();
    // let mb = b.unwrap_or(binding);

    let ma = a.as_ref().unwrap();
    let mb = b.as_ref().unwrap();

    // Collect all keys
    let keys: BTreeSet<_> = ma.keys().chain(mb.keys()).collect();

    for key in keys {
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
pub fn create_json_patch<T>(current_state: &Value, desired_state: &Value) -> Value
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

    // Convert the patch to a JSON value
    serde_json::to_value(patch).unwrap()
}

/// serialize the object to the file format specified
pub fn serialize_object<T: serde::Serialize>(object: &T, file_format: &FileFormat) -> String {
    match file_format {
        FileFormat::Yaml => serde_yaml::to_string(object)
            .map_err(|e| {
                println!("Failed to serialize object: {:?}", e);
                std::process::exit(1);
            })
            .unwrap(),
        FileFormat::Json => serde_json::to_string_pretty(object)
            .map_err(|e| {
                println!("Failed to serialize object: {:?}", e);
                std::process::exit(1);
            })
            .unwrap(),
        FileFormat::Toml => toml::to_string_pretty(object)
            .map_err(|e| {
                println!("Failed to serialize object: {:?}", e);
                std::process::exit(1);
            })
            .unwrap(),
    }
}

// deserialize the project from the format specified
///
/// # Arguments
/// FileFormat: The format of the file to be deserialized
/// object: The object to be deserialized
pub fn deserialize_object<T: serde::de::DeserializeOwned>(
    object: &str,
    file_format: &FileFormat,
) -> T {
    match file_format {
        FileFormat::Yaml => serde_yaml::from_str(object).unwrap(),
        FileFormat::Json => serde_json::from_str(object).unwrap(),
        FileFormat::Toml => toml::de::from_str(object).unwrap(),
    }
}

pub fn file_format_from_extension(extension: &str) -> FileFormat {
    match extension {
        "yml" => FileFormat::Yaml,
        "json" => FileFormat::Json,
        "toml" => FileFormat::Toml,
        _ => FileFormat::Json,
    }
}

pub fn file_format_from_path(path: &Path) -> FileFormat {
    match path.extension() {
        Some(ext) => file_format_from_extension(ext.to_str().unwrap()),
        None => FileFormat::Json,
    }
}

pub fn file_extension_from_format(file_format: &FileFormat) -> String {
    match file_format {
        FileFormat::Yaml => "yml".to_string(),
        FileFormat::Json => "json".to_string(),
        FileFormat::Toml => "toml".to_string(),
    }
}

pub fn file_format(file_format: &str) -> FileFormat {
    match file_format {
        "yaml" => FileFormat::Yaml,
        "json" => FileFormat::Json,
        "toml" => FileFormat::Toml,
        _ => FileFormat::Json,
    }
}

pub enum FileFormat {
    Yaml,
    Json,
    Toml,
}

pub enum ResourceVersionMatch {
    Exact,
    NotOlderThan,
}

impl ResourceVersionMatch {

    fn as_str(&self) -> &'static str {
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
