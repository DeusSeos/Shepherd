// This file will contain all the functions that will be used to interact and extract from the Rancher API
pub mod cluster;
pub mod project;
pub mod rt;
pub mod prtb;
pub mod git;

use std::path::PathBuf;

use cluster::Cluster;
use project::Project;
use prtb::ProjectRoleTemplateBinding;
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
// the function will return the path to the folder

pub async fn download_current_configuration(configuration: &Configuration, path: &PathBuf, file_format: FileFormat) {
    // Get the current configuration from the Rancher API
    let rancher_cluster = cluster::get_clusters(configuration).await.map_err(|e| {
        println!("Failed to get clusters: {:?}", e);
        std::process::exit(1);
    }).unwrap();
    let rancher_role_templates = rt::get_role_templates(configuration).await.map_err(|e| {
        println!("Failed to get role templates: {:?}", e);
        std::process::exit(1);
    }).unwrap();

    // Create the base folder if it does not exist. Base folder will be the path provided and the endpoint url
    // for example: /tmp/rancher_config/https://rancher.rd.localhost

    let base_path = path.join(configuration.base_path.replace("https://", "").replace("/", "_"));
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
    let role_templates: Vec<rt::RoleTemplate> = rancher_role_templates
        .items
        .into_iter()
        .map(|role_template| {
            // try to convert the IoCattleManagementV3RoleTemplate to our simple RoleTemplate struct
            // if it fails, return an error
            role_template.try_into().map_err(|e| {
                println!("Failed to convert role template: {:?}", e);
                std::process::exit(1);
            }).unwrap()
        })
        .collect::<Vec<rt::RoleTemplate>>();


    // Loop through the role templates and save them to the folder
    for role_template in &role_templates {
        // save the role template to the folder
        let role_template_file = role_template_path.join(format!("{}.{}", role_template.id, file_extension_from_format(&file_format)));
        let _ = std::fs::write(&role_template_file, serialize_object(role_template, &file_format)).map_err(|e| {
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
            cluster.try_into().map_err(|e| {
                println!("Failed to convert cluster: {:?}", e);
                std::process::exit(1);
            }).unwrap()
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
        let cluster_file = cluster_path.join(format!("{}.{}", cluster.id, file_extension_from_format(&file_format)));
        let _ = std::fs::write(&cluster_file, serialize_object(cluster, &file_format)).map_err(|e| {
            println!("Failed to write file: {:?}", e);
            std::process::exit(1);
        });


        // fetch the projects for the cluster
        let rancher_projects = project::get_projects(configuration, &cluster.id).await.map_err(|e| {
            println!("Failed to get projects: {:?}", e);
            std::process::exit(1);
        }).unwrap();
        // convert the items to our simple struct for each object using the try_from method
        let projects: Vec<Project> = rancher_projects
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


            let rancher_project_role_template_bindings = prtb::get_namespaced_project_role_template_bindings(configuration, &project.id).await.map_err(|e| {
                println!("Failed to get project role template bindings: {:?}", e);
                std::process::exit(1);
            }).unwrap();
        
            
            // TODO: convert the conversion to a generic function
            let prtbs: Vec<ProjectRoleTemplateBinding> = rancher_project_role_template_bindings
            .items
            .into_iter()
            .map(|prtb| {
                // try to convert the IoCattleManagementV3ProjectRoleTemplateBinding to our simple ProjectRoleTemplateBinding struct
                // if it fails, return an error
                prtb.try_into().map_err(|e| {
                    println!("Failed to convert project role template binding: {:?}", e);
                    std::process::exit(1);
                }).unwrap()
            })
            .collect::<Vec<prtb::ProjectRoleTemplateBinding>>();

            // Loop through the project role template bindings and save them to the folder
            for prtb in &prtbs {
                // save the project role template binding to the project folder
                let prtb_file = project_path.join(format!("{}.{}", prtb.id, file_extension_from_format(&file_format)));
                let _ = std::fs::write(&prtb_file, serialize_object(prtb, &file_format)).map_err(|e| {
                    println!("Failed to write file: {:?}", e);
                    std::process::exit(1);
                });
            }
        }
    }
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
pub fn load_project(base_path: &PathBuf, endpoint_url: &str, cluster_id: &str, project_id: &str, file_format: FileFormat) -> Project {
    // create the path to the project
    let project_path = base_path.join(endpoint_url.replace("https://", "").replace("/", "_")).join(cluster_id).join(project_id);
    // check if the path exists
    if !project_path.exists() {
        println!("Project path does not exist");
        std::process::exit(1);
    }
    // read the file from the path
    let project_file = project_path.join(format!("{}.{}", project_id, file_extension_from_format(&file_format)));
    // check if the file exists
    if !project_file.exists() {
        println!("Project file does not exist");
        std::process::exit(1);
    }
    // read the file and deserialize it
    let project_file_content = std::fs::read_to_string(&project_file).map_err(|e| {
        println!("Failed to read file: {:?}", e);
        std::process::exit(1);
    }).unwrap();
    
    deserialize_object(&project_file_content, file_format)
}




// serialize the object to the format specified
pub fn serialize_object<T: serde::Serialize>(object: &T, file_format: &FileFormat) -> String {
    match file_format {
        FileFormat::Yaml => serde_yaml::to_string(object).map_err(|e| {
            println!("Failed to serialize object: {:?}", e);
            std::process::exit(1);
        }).unwrap(),
        FileFormat::Json => serde_json::to_string_pretty(object).map_err(|e| {
            println!("Failed to serialize object: {:?}", e);
            std::process::exit(1);
        }).unwrap(),
        FileFormat::Toml => toml::to_string_pretty(object).map_err(|e| {
            println!("Failed to serialize object: {:?}", e);
            std::process::exit(1);
        }).unwrap(),
    }
}


// deserialize the project from the format specified
/// 
/// # Arguments
/// FileFormat: The format of the file to be deserialized
/// object: The object to be deserialized

pub fn deserialize_object<T: serde::de::DeserializeOwned>(object: &str, file_format: FileFormat) -> T {
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

pub fn file_format_from_path(path: &PathBuf) -> FileFormat {
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