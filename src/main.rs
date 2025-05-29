#![allow(unused_variables)]
// #![allow(unused_imports)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};

use rancher_cac::api::client::ShepherdClient;
use rancher_cac::api::config;

use rancher_cac::error::{handle_result_collection, AppError};

use rancher_cac::models::{MinimalObject, ObjectType};
use rancher_cac::utils::file::{
    get_minimal_object_from_contents, is_directory_empty, write_back_objects, FileFormat,
};
use rancher_cac::utils::git::{
    commit_changes, get_deleted_files_and_contents, get_modified_files, get_new_uncommited_files,
    init_git_repo_with_main_branch,
};

use rancher_cac::modify::{compare_and_update_configurations, create_objects, delete_objects};

use rancher_cac::download_current_configuration;

use rancher_client::apis::configuration::{Configuration};
use tokio::time::interval;

use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

// const RETRY_DELAY: Duration = Duration::from_millis(200);
// const LOOP_INTERVAL: Duration = Duration::from_secs(60);

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
async fn run_sync(
    client_config: Arc<Configuration>,
    config_folder_path: &Path,
    remote_url: &str,
    file_format: FileFormat,
    cluster_ids: Vec<String>,
    loop_interval: u64,
    retry_delay: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a interval ticker
    let mut interval_timer = interval(Duration::from_secs(loop_interval));

    let retry_delay = Duration::from_millis(retry_delay);

    loop {
        interval_timer.tick().await;

        info!("Starting scheduled run at {}", chrono::Utc::now());

        let download = download_required(&config_folder_path).await?;
        if download {
            download_current_configuration(
                &client_config,
                &config_folder_path,
                &file_format,
            )
            .await?;
            
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

            let update_objects = compare_and_update_configurations(
                client_config.clone(),
                &config_folder_path,
                &cluster_id,
                &file_format,
            )
            .await;
            let created_objects =
                create_objects(client_config.clone(), new_files, 10, 5, retry_delay).await;

            let (successes, mut errors) = handle_result_collection(created_objects);

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
            let (_, delete_errors) = handle_result_collection(deleted_objects);

            errors.extend(delete_errors);
        }
        info!("Run complete at {}", chrono::Utc::now());
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //Setup logging

    init_tracing();

    // get home path and concatenate with .config/shepherd/config.toml
    let home_path = std::env::var("HOME")
        .map_err(|_| AppError::Other("HOME environment variable not set".to_string()))?;
    let app_config_path = home_path + "/.config/shepherd/config.toml";
    let app_config = config::ShepherdConfig::from_file(&app_config_path)
        .context(format!("Failed to load config file: {}", app_config_path));

    // if we have an error spit out the error and exit
    let app_config = match app_config {
        Ok(config) => config,
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };

    debug!("App config: {}", app_config);

    let client = ShepherdClient::new(&app_config.endpoint_url, &app_config.token, true);
    let config_folder_path = app_config.rancher_config_path;
    let remote_url = app_config.remote_git_url.unwrap();
    let file_format = app_config.file_format;
    let client_config = client.config.clone();
    let cluster_ids = app_config.cluster_names.unwrap();
    // in seconds
    let loop_interval = app_config.loop_interval;
    // in milliseconds
    let retry_delay = app_config.retry_delay;
    

    run_sync(
        client_config,
        &config_folder_path,
        &remote_url,
        file_format,
        cluster_ids,
        loop_interval,
        retry_delay,
    )
    .await?;

    Ok(())
}

async fn download_required(config_folder_path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    match is_directory_empty(config_folder_path).await {
        Ok(true) => {
            info!("Directory is empty: {}", config_folder_path.display());
            // Handle empty directory case
            Ok(true)
        }
        Ok(false) => {
            info!("Directory is not empty: {}", config_folder_path.display());
            // Handle non-empty directory case
            Ok(false)
        }
        Err(e) => {
            error!("Error checking directory: {}", e);
            // Handle error
            Ok(true)
        }
    }
}
