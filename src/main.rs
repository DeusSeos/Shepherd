#![allow(unused_variables)]
// #![allow(unused_imports)]

use std::path::PathBuf;

use rancher_cac::api::client::ShepherdClient;
use rancher_cac::api::config;
use rancher_cac::utils::file::{
    get_minimal_object_from_contents, write_back_objects,
};
use rancher_cac::utils::git::{
    commit_changes, get_deleted_files_and_contents, get_modified_files,
    get_new_uncommited_files, init_git_repo_with_main_branch,
};
use rancher_cac::models::MinimalObject;
use rancher_cac::modify::{compare_and_update_configurations, create_objects, delete_objects};
use rancher_cac::{
    download_current_configuration, models::CreatedObject, models::ObjectType,
};

use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;



fn init_tracing() {
    // Initialize the tracing subscriber using RUST_LOG environment variable
    // ignore statements not from this crate
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_file(true)
        .with_line_number(true)
        .init();
}

// /*
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    // get home path and concatenate with .config/shepherd/config.toml
    let home_path = std::env::var("HOME").unwrap();
    let app_config_path = home_path + "/.config/shepherd/config.toml";
    let app_config = config::ShepherdConfig::from_file(&app_config_path).unwrap();

    debug!("App config: {:#?}", app_config);

    let client = ShepherdClient::new(&app_config.endpoint_url, &app_config.token, true);

    let config_folder_path = app_config.rancher_config_path;

    let remote_url = app_config.remote_git_url.unwrap();

    let file_format = app_config.file_format;

    let client_config = client.config.clone();

    let cluster_ids = app_config.cluster_names.unwrap();

    let download = false;

    if download {
        // Download the current configuration from the Rancher API
        download_current_configuration(&client.config, &config_folder_path, &file_format).await?;
        // TODO: generate a short hand version of the updates made to the configuration

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
        let cluster_id = cluster_ids[0].clone();

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

        let update_objects = compare_and_update_configurations(client_config.clone(), &config_folder_path, &cluster_id, &file_format).await;
        let created_objects = create_objects(client_config.clone(), new_files, file_format).await;

        // Separate errors and successes from object creation results
        let mut errors = Vec::new();
        let mut successes: Vec<(PathBuf, CreatedObject)> = Vec::new();
        for result in created_objects {
            match result {
                Ok((file_path, created_object)) => successes.push((file_path, created_object)),
                Err(err) => errors.push(err),
            }
        }

        // Write back the successfully created objects
        write_back_objects(successes, file_format).await?;

        let mut objects_to_delete: Vec<(ObjectType, MinimalObject)> = Vec::new();

        for (object_type, path, contents) in deleted_files_and_contents {
            let minimal_object =
                get_minimal_object_from_contents(object_type, &contents, &file_format)
                    .await
                    .unwrap();
            objects_to_delete.push((object_type, minimal_object));
        }

        let deleted_objects = delete_objects(client_config.clone(), objects_to_delete).await;


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

    Ok(())
}

