// This file will contain all the functions that will be used to interact and extract from the Rancher API
pub mod cluster;
pub mod project;
pub mod rt;
pub mod prtb;

use std::path::PathBuf;

use cluster::Cluster;
use project::Project;
use rancher_client::{apis::configuration::{ApiKey, Configuration}, models::IoCattleManagementv3Cluster};

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
// the function will return the path to the folder

pub async fn download_current_configuration(configuration: Configuration, path: PathBuf, file_format: FileFormat) {
    // Get the current configuration from the Rancher API
    let rancher_cluster = cluster::get_clusters(&configuration).await.map_err(|e| {
        println!("Failed to get clusters: {:?}", e);
        std::process::exit(1);
    }).unwrap();
    let rancher_role_templates = rt::get_role_templates(&configuration).await.map_err(|e| {
        println!("Failed to get role templates: {:?}", e);
        std::process::exit(1);
    }).unwrap();

    let rancher_project_role_template_bindings = prtb::get_project_role_template_bindings(&configuration).await.map_err(|e| {
        println!("Failed to get project role template bindings: {:?}", e);
        std::process::exit(1);
    }).unwrap();

    // Create the base folder if it does not exist 
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path).map_err(|e| {
            println!("Failed to create folder: {:?}", e);
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
            cluster.try_into().map_err(|e| {
                println!("Failed to convert cluster: {:?}", e);
                std::process::exit(1);
            }).unwrap()
        })
        .collect::<Vec<Cluster>>();

    // Loop through the clusters and save them to the folder
    for cluster in &clusters {
        // create the folder for the cluster if it does not exist
        let cluster_path = path.join(cluster.id.clone());
        if !cluster_path.exists() {
            let _ = std::fs::create_dir_all(&cluster_path).map_err(|e| {
                println!("Failed to create folder: {:?}", e);
                std::process::exit(1);
            });
        }

        // save the cluster to the folder
        let cluster_file = cluster_path.join(format!("{}.{}", cluster.id, file_extension_from_format(&file_format)));
        let _ = std::fs::write(&cluster_file, serialize_object(cluster, &file_format)).map_err(|e| {
            println!("Failed to write file: {:?}", e);
            std::process::exit(1);
        });
        // fetch the projects for the cluster
        let projects = project::get_projects(&configuration, &cluster.id).await.map_err(|e| {
            println!("Failed to get projects: {:?}", e);
            std::process::exit(1);
        }).unwrap();
        // convert the items to our simple struct for each object using the try_from method
        let projects: Vec<Project> = projects
            .items
            .into_iter()
            .map(|project| {
                // try to convert the IoCattleManagementV3Project to our simple Project struct
                // if it fails, return an error
                project.try_into().map_err(|e| {
                    println!("Failed to convert project: {:?}", e);
                    std::process::exit(1);
                }).unwrap()
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
            let project_file = project_path.join(format!("{}.{}", project.id, file_extension_from_format(&file_format)));
            let _ = std::fs::write(&project_file, serialize_object(project, &file_format)).map_err(|e| {
                println!("Failed to write file: {:?}", e);
                std::process::exit(1);
            });

            // pop the project path
        }
    }


    // // Save the configuration to the folder
    // // Save the clusters to the folder
    // for cluster in clusters.items {
    //     // 
    // }

}


// serialize the object to the format specified
pub fn serialize_object<T: serde::Serialize>(object: &T, file_format: &FileFormat) -> String {
    match file_format {
        FileFormat::Yaml => serde_yaml::to_string(object).map_err(|e| {
            println!("Failed to serialize object: {:?}", e);
            std::process::exit(1);
        }).unwrap(),
        FileFormat::Json => serde_json::to_string(object).map_err(|e| {
            println!("Failed to serialize object: {:?}", e);
            std::process::exit(1);
        }).unwrap(),
        FileFormat::Toml => toml::to_string(object).map_err(|e| {
            println!("Failed to serialize object: {:?}", e);
            std::process::exit(1);
        }).unwrap(),
    }
}


// deserialize the object from the format specified
pub fn deserialize_object<T: serde::de::DeserializeOwned>(object: &str, file_format: FileFormat) -> T {
    match file_format {
        FileFormat::Yaml => serde_yaml::from_str(object).unwrap(),
        FileFormat::Json => serde_json::from_str(object).unwrap(),
        FileFormat::Toml => toml::de::from_str(object).unwrap(),
    }
}


pub fn file_format_from_extension(extension: &str) -> FileFormat {
    match extension {
        "yaml" => FileFormat::Yaml,
        "json" => FileFormat::Json,
        "toml" => FileFormat::Toml,
        _ => FileFormat::Json,
    }
}

pub fn file_format_from_path(path: &PathBuf) -> FileFormat {
    match path.extension() {
        Some(ext) => file_format_from_extension(ext.to_str().unwrap()),
        None => FileFormat::Json,
    }
}

pub fn file_extension_from_format(file_format: &FileFormat) -> String {
    match file_format {
        FileFormat::Yaml => "yaml".to_string(),
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