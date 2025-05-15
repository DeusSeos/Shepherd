#![allow(unused_variables)]
#![allow(unused_imports)]

use std::collections::HashMap;
use std::path::PathBuf;

use rancher_cac::cluster::Cluster;
use rancher_cac::config::RancherClusterConfig;
use rancher_cac::file::FileFormat;
use rancher_cac::prtb::{create_project_role_template_binding, ProjectRoleTemplateBinding};
use rancher_cac::rt::{create_role_template, RoleTemplate};
use rancher_cac::update::compare_and_update_configurations;
use rancher_cac::{ create_objects, download_current_configuration, load_configuration, load_configuration_from_rancher, load_object, rancher_config_init, CreatedObject, ObjectType};
use rancher_cac::diff::{compute_cluster_diff, create_json_patch};
use rancher_cac::git::{commit_changes, get_new_uncommited_files, init_git_repo_with_main_branch, push_repo_to_remote};
use rancher_cac::project::{load_project, create_project, find_project, get_projects, show_project_diff, show_text_diff, update_project, Project, PROJECT_EXCLUDE_PATHS};


use rancher_client::models::{IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding, IoCattleManagementv3RoleTemplate};
use reqwest_middleware::ClientBuilder;
use serde_json::json;


/* #[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // TODO: change this to use URL and token fetched from our custom config file
    let mut configuration = rancher_config_init(
        "https://rancher.rd.localhost",
        "token-xxzn4:mlcl7q4m2vl6mq8hfzdffh5f5fh4wfzqqhzbm52bqzkpmhdg2c7bf7",
    );

    // modify the configuration client to allow self-signed certificates, TODO: Remove this when we have proper certificate handling
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    configuration.client = ClientBuilder::new(client).build();

    // TODO: change this to use path fetched from our custom config file
    let path = std::path::PathBuf::from("/Users/dc/Documents/Rust/rancher_config");
    
    // TODO: change this to use remote url fetched from our custom config file
    let remote_url = "git@github.com:DeusSeos/rancher_config.git";
    
    // TODO: change this to use format fetched from our custom config file
    let file_format = FileFormat::Yaml;

    let download = true;

    if download {
        // Download the current configuration from the Rancher API
        download_current_configuration(&configuration, &path, &file_format).await?;

        // set up the remote url to be git@github.com/DeusSeos/rancher_config.git
        let remote_url = "git@github.com:DeusSeos/rancher_config.git";

        // Initialize a git repository in the path or if error, commit a change with current datetime
        init_git_repo_with_main_branch(&path, &remote_url).unwrap_or_else(|_| {
            // commit a change with current datetime
            let now = chrono::Utc::now();
            let datetime = now.format("%Y-%m-%d %H:%M:%S").to_string();
            let message = format!("Updated configuration at {}", datetime);
            commit_changes(&path, &message).unwrap();
            println!(
                "Error initializing git repository, committed changes with message: {}",
                message
            );
        });
    }

    let new_files = get_new_uncommited_files(&path).await?;
    println!("New files: {:?}", new_files);

    // push the repo to the remote
    // push_repo_to_remote(&path, &remote_url).unwrap();

    Ok(())
} */



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // TODO: change this to use URL and token fetched from our custom config file
    let mut configuration = rancher_config_init(
        "https://rancher.rd.localhost",
        "token-xxzn4:mlcl7q4m2vl6mq8hfzdffh5f5fh4wfzqqhzbm52bqzkpmhdg2c7bf7",
    );

    // modify the configuration client to allow self-signed certificates, TODO: Remove this when we have proper certificate handling
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    configuration.client = ClientBuilder::new(client).build();

    // TODO: change this to use path fetched from our custom config file
    let path = std::path::PathBuf::from("/Users/dc/Documents/Rust/rancher_config");
    
    // TODO: change this to use remote url fetched from our custom config file
    let remote_url = "git@github.com:DeusSeos/rancher_config.git";
    
    // TODO: change this to use format fetched from our custom config file
    let file_format = FileFormat::Yaml;

    let cluster_id = "local";

    let new_files = get_new_uncommited_files(&path).await?;
    println!("New files: {:?}", new_files);

    let created_objects = create_objects(&configuration, new_files, &file_format).await;
    for object_type in created_objects {
        let error = object_type.map(|created_object| match created_object {
            CreatedObject::ProjectRoleTemplateBinding(created) => println!("Created PRTB: {:#?}", created),
            CreatedObject::Project(created) => println!("Created Project: {:#?}", created),
            CreatedObject::RoleTemplate(created) => println!("Created Role Template: {:#?}", created),
        });

        match error {
            Err(e) => eprintln!("Error creating object: {:?}", e),
            _ => {}
        }
    }




    // push the repo to the remote
    // push_repo_to_remote(&path, &remote_url).unwrap();

    Ok(())
}