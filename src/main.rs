#![allow(unused_variables)]
#![allow(unused_imports)]

use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use json_patch::jsonptr::delete;
use rancher_cac::cluster::Cluster;
use rancher_cac::config::RancherClusterConfig;
use rancher_cac::diff::{compute_cluster_diff, create_json_patch};
use rancher_cac::file::write_object_to_file;
use rancher_cac::file::{
    get_minimal_object_from_contents, get_minimal_object_from_path, write_back_objects, FileFormat,
};
use rancher_cac::git::{
    commit_changes, get_deleted_files, get_deleted_files_and_contents, get_modified_files,
    get_new_uncommited_files, init_git_repo_with_main_branch, push_repo_to_remote,
};
use rancher_cac::models::MinimalObject;
use rancher_cac::modify::{compare_and_update_configurations, create_objects, delete_objects};
use rancher_cac::project::{
    create_project, find_project, get_projects, load_project, show_project_diff, show_text_diff,
    update_project, Project, PROJECT_EXCLUDE_PATHS,
};
use rancher_cac::prtb::{create_project_role_template_binding, ProjectRoleTemplateBinding};
use rancher_cac::rt::{create_role_template, RoleTemplate};
use rancher_cac::{
    download_current_configuration, load_configuration, load_configuration_from_rancher,
    load_object, models::CreatedObject, models::ObjectType, rancher_config_init,
};

use rancher_client::apis::configuration;
use serde::de;
use tracing::{debug, error, info};
use tracing_subscriber::fmt::format::FmtSpan;

use ::futures::future::join_all;
use rancher_client::models::{
    IoCattleManagementv3Project, IoCattleManagementv3ProjectRoleTemplateBinding,
    IoCattleManagementv3RoleTemplate,
};
use reqwest_middleware::ClientBuilder;
use serde_json::json;
use tokio::sync::futures;
use tokio::task::JoinHandle;

// /*
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
        .init();

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
    let config_folder_path = std::path::PathBuf::from("/Users/dc/Documents/Rust/rancher_config");

    // TODO: change this to use remote url fetched from our custom config file
    let remote_url = "git@github.com:DeusSeos/rancher_config.git";

    // TODO: change this to use format fetched from our custom config file
    let file_format = FileFormat::Yaml;

    let download = true;

    if download {
        // Download the current configuration from the Rancher API
        download_current_configuration(&configuration, &config_folder_path, &file_format).await?;
        // TODO: generate a short hand version of the updates made to the configuration

        // set up the remote url to be git@github.com/DeusSeos/rancher_config.git
        let remote_url = "git@github.com:DeusSeos/rancher_config.git";

        // Initialize a git repository in the path or if error, commit a change with current datetime
        init_git_repo_with_main_branch(&config_folder_path, &remote_url).unwrap_or_else(|_| {
            // commit a change with current datetime
            let now = chrono::Utc::now();
            let datetime = now.format("%Y-%m-%d %H:%M:%S").to_string();
            let message = format!("Updated configuration at {}", datetime);
            commit_changes(&config_folder_path, &message).unwrap();
            println!(
                "Git repository initialized or already exists, committed changes with message: {}",
                message
            );
        });
    } else {

        // TODO: change this to use cluster id fetched from our custom config file
        let cluster_id = "local";

        let new_files = get_new_uncommited_files(&config_folder_path).await?;

        let modified_files = get_modified_files(&config_folder_path).await?;

        let deleted_files_and_contents =
            get_deleted_files_and_contents(&config_folder_path).await?;

        info!("New files: {:?}", new_files);

        info!("Modified files: {:?}", modified_files);

        info!(
            "Deleted files: {:?}",
            deleted_files_and_contents
                .iter()
                .map(|(object_type, path, _)| (object_type, path))
                .collect::<Vec<_>>()
        );

        let configuration = Arc::new(configuration);

        let update_objects = compare_and_update_configurations(
            configuration.clone(),
            &config_folder_path,
            cluster_id,
            &file_format,
        )
        .await;

        let created_objects = create_objects(configuration.clone(), new_files, file_format).await;

        // Separate errors and successes from object creation results
        let mut errors: Vec<Box<dyn Error>> = Vec::new();
        let mut successes: Vec<(PathBuf, CreatedObject)> = Vec::new();
        for result in created_objects {
            match result {
                Ok((file_path, created_object)) => successes.push((file_path, created_object)),
                Err(err) => errors.push(err),
            }
        }

        // Write back the successfully created objects
        write_back_objects(successes, file_format).await;

        let mut objects_to_delete: Vec<(ObjectType, MinimalObject)> = Vec::new();

        for (object_type, path, contents) in deleted_files_and_contents {
            let minimal_object =
                get_minimal_object_from_contents(object_type, &contents, &file_format)
                    .await
                    .unwrap();
            objects_to_delete.push((object_type, minimal_object));
        }

        let deleted_objects = delete_objects(configuration.clone(), objects_to_delete).await;


        for result in deleted_objects {
            match result {
                Ok(_) => (),
                Err(err) => errors.push(err),
            }
        }


        // Report errors, if any
        if !errors.is_empty() {
            for err in errors {
                error!("Error for object: {:#?}", err);
            }
        }
    }

    // push the repo to the remote
    // push_repo_to_remote(&path, &remote_url).unwrap();

    Ok(())
}

