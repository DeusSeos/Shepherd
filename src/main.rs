#![allow(unused_variables)]
#![allow(unused_imports)]

use std::collections::HashMap;
use std::path::PathBuf;

use rancher_cac::cluster::Cluster;
use rancher_cac::config::RancherClusterConfig;
use rancher_cac::file::FileFormat;
use rancher_cac::update::compare_and_update_configurations;
use rancher_cac::{ download_current_configuration, load_configuration, load_configuration_from_rancher, rancher_config_init};
use rancher_cac::diff::{compute_cluster_diff, create_json_patch};
use rancher_cac::git::{commit_changes, init_git_repo_with_main_branch, push_repo_to_remote};
use rancher_cac::project::{find_project, get_projects, show_project_diff, show_text_diff, update_project, Project, PROJECT_EXCLUDE_PATHS};


use rancher_client::models::IoCattleManagementv3Project;
use reqwest_middleware::ClientBuilder;
use serde_json::json;



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




    // push the repo to the remote
    // push_repo_to_remote(&path, &remote_url).unwrap();

    Ok(())
}



/*
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>  {
    // 1️⃣ build your config (fill in base_path / api_key as appropriate)
    let mut configuration = rancher_config_init(
        "https://rancher.rd.localhost",
        "token-xxzn4:mlcl7q4m2vl6mq8hfzdffh5f5fh4wfzqqhzbm52bqzkpmhdg2c7bf7",
    );

    // modify the configuration client to allow self-signed certificates
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    configuration.client = ClientBuilder::new(client).build();

    // // // 2️⃣ call with a fieldSelector
    // let resp = list_management_cattle_io_v3_namespaced_project(
    //     &configuration,
    //     "local",               // namespace
    //     None,                  // pretty
    //     None,                  // allow_watch_bookmarks
    //     None,                  // continue
    //     None, // field_selector
    //     Some("another=two"),                  // label_selector
    //     None, None, None, None, None, None,
    // )
    // .await?;


    // // 2️⃣ call with resourceVersionMatch and resourceVersion
    // let resp = get_projects(
    //     &configuration,
    //     "local",                        // cluster_id
    //     None,                           // field_selector
    //     None,                           // label_selector
    //     None,                          // limit
    //     Some("1311731"),                           // resource_version
    //     Some(ResourceVersionMatch::Exact), // resource_version_match
    //     None,                          // continue
    // )
    // .await?;

    // // 2️⃣ call with resourceVersion 
    // // let resp = read_management_cattle_io_v3_namespaced_project(&configuration, "p-w82pc" , "local", Some("true"), Some("1288128") ).await?;


    // println!("{:#?}", resp);

    // load the cluster config
    let path_buf = PathBuf::from("/Users/dc/Documents/Rust/rancher_config");
    let path = path_buf.as_path();

    let endpoint_url = configuration.base_path.clone();
    let file_format = FileFormat::Yaml;
    let cluster_id = "local";

    
    let resp = compare_and_update_configurations(&configuration, path, cluster_id, &file_format).await;
    println!("{:#?}", resp);


    Ok(())
}
*/