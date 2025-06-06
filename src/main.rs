use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use shepherd::api::client::ShepherdClient;
use shepherd::api::config::ShepherdConfig;
use shepherd::error::{handle_result_collection, AppError};
use shepherd::models::{MinimalObject, ObjectType};
use shepherd::utils::file::{
    get_minimal_object_from_contents, is_directory_empty, write_back_objects, FileFormat,
};
use shepherd::utils::git::{
    commit_changes, get_deleted_files_and_contents, get_modified_files, get_new_uncommited_files,
    init_git_repo_with_main_branch, pull_changes, push_changes, resolve_conflicts, safe_clone_repository, GitAuth, GitError,
};
use shepherd::modify::{compare_and_update_configurations, create_objects, delete_objects};
use shepherd::download_current_configuration;
use rancher_client::apis::configuration::Configuration;


use anyhow::Result;
use git2::Repository;
use tokio::time::interval;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;
use walkdir::WalkDir;

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


/// Runs the main loop of the configuration synchronization process.
///
/// This function will download the current configuration from the Rancher API if
/// the local folder is empty. It will then initialize a git repository and
/// download the current configuration.
///
/// After that, it will enter a loop where it will:
///
/// 1. Pull changes from the remote repository
/// 2. Commit local changes
/// 3. Push changes to the remote repository
/// 4. Update the objects in the Rancher API if the local files have changed
/// 5. Create new objects in the Rancher API if new files have been added
/// 6. Delete objects in the Rancher API if files have been deleted
///
/// The function will run indefinitely until it is stopped.
///
/// It takes the following parameters:
///
/// - `client_config`: The configuration for the Rancher API client
/// - `config_folder_path`: The path to the folder where the configuration files are stored
/// - `remote_url`: The URL of the remote git repository
/// - `file_format`: The file format of the configuration files
/// - `cluster_ids`: A vector of cluster IDs to synchronize the configuration for
/// - `loop_interval`: The interval in seconds to wait between each run
/// - `retry_delay`: The delay in milliseconds to wait before retrying an operation
/// - `branch`: The branch to use in the remote repository
/// - `auth_method`: The authentication method to use for the remote repository
async fn run_sync(
    client_config: Arc<Configuration>,
    config_folder_path: &Path,
    remote_url: &str,
    file_format: FileFormat,
    cluster_ids: Vec<String>,
    loop_interval: u64,
    retry_delay: u64,
    branch: &str,
    auth_method: GitAuth,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a interval ticker
    let mut interval_timer = interval(Duration::from_secs(loop_interval));

    let retry_delay = Duration::from_millis(retry_delay);

    let download_required = download_required(config_folder_path, remote_url, &auth_method).await;

    match download_required {
        Ok(true) => {
            info!("Downloading required");

            let _ = init_git_repo_with_main_branch(&config_folder_path, &remote_url, branch)
                .map_err(|e| {
                    error!("Failed to initialize git repo: {}", e);
                    e
                });

            let _ =
                download_current_configuration(&client_config, config_folder_path, &file_format)
                    .await;
            // init git repo
        }
        Ok(false) => {
            info!("Downloading not required");
        }
        Err(e) => {
            error!("Failed to check if download is required: {}", e);
        }
    }

    loop {
        interval_timer.tick().await;

        info!("Starting scheduled run at {}", chrono::Utc::now());

        // Initialize repository if it doesn't exist
        let repo = match Repository::open(&config_folder_path) {
            Ok(repo) => repo,
            Err(_) => {
                info!("Repository not found, initializing...");
                init_git_repo_with_main_branch(&config_folder_path, &remote_url, branch)?;
                Repository::open(&config_folder_path).map_err(|e| {
                    error!("Failed to open repository: {}", e);
                    e
                })?
            }
        };

        info!("Repository found");
        info!("Pulling changes...");
        // Pull changes
        match pull_changes(&repo, branch, &auth_method) {
            Ok(_) => info!("Successfully pulled changes"),
            Err(e) => {
                error!("Failed to pull changes: {}", e);
                // Handle merge conflicts
                resolve_conflicts(&repo, branch)?;
            }
        }

        // Commit local changes
        let now = chrono::Utc::now();
        let datetime = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let message = format!("Updated configuration at {}", datetime);
        commit_changes(&config_folder_path, &message)?;

        // Push changes
        match push_changes(&repo, branch, &auth_method) {
            Ok(_) => info!("Successfully pushed changes"),
            Err(e) => error!("Failed to push changes: {}", e),
        }

        // let cluster_id = cluster_ids[0].clone();

        for cluster_id in cluster_ids.iter() {
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

            let _update_objects = compare_and_update_configurations(
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

            for (object_type, _path, contents) in deleted_files_and_contents {
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

pub async fn is_repo_effectively_empty(repo: &Repository) -> Result<bool, GitError> {
    let workdir = repo.workdir().ok_or_else(|| {
        GitError::Other("Repository has no working directory (bare repo?)".to_string())
    })?;

    let root = workdir.to_path_buf();

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|e| e.file_name() != ".git")
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path: PathBuf = entry.path().to_path_buf();

        // Check if file is ignored
        let is_ignored = repo.status_should_ignore(&path).unwrap_or(false);
        if !is_ignored {
            return Ok(false);
        }
    }

    Ok(true)
}


    /// Checks if a download of the remote repository is required by checking if the local config
    /// folder is empty. If the folder is empty, it clones the remote repository into the folder
    /// and checks if the repository is empty after cloning. If the repository is empty, it returns
    /// true, indicating that a download is required. If the folder is not empty, it returns false.
    /// If an error occurs during the check, it returns true.
    ///
    /// # Arguments
    /// * `config_folder_path` - Path to the local config folder
    /// * `remote_url` - URL of the remote repository
    /// * `auth_method` - Authentication method to use when cloning the repository
    ///
    /// # Returns
    /// * `Result<bool, Box<dyn std::error::Error>>` - Result indicating whether a download is required
async fn download_required(
    config_folder_path: &Path,
    remote_url: &str,
    auth_method: &GitAuth,
) -> Result<bool, Box<dyn std::error::Error>> {
    match is_directory_empty(config_folder_path).await {
        Ok(true) => {
            info!("Directory is empty: {}", config_folder_path.display());
            // Clone the remote repository into the empty directory
            let cloned = safe_clone_repository(config_folder_path, remote_url, auth_method).await;
            // handle error
            match cloned {
                Ok(repo) => {
                    info!("Repository cloned successfully: {}", repo.path().display());

                    let repo = Repository::open(config_folder_path)?;
                    if is_repo_effectively_empty(&repo).await? {
                        info!("Repository is empty after cloning (ignoring .git and .gitignored files)");
                        return Ok(true);
                    }

                    Ok(false)
                }
                Err(e) => {
                    error!("Failed to clone repository: {}", e);
                    Ok(true)
                }
            }
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //Setup logging

    init_tracing();

    // get home path and concatenate with .config/shepherd/config.toml
    let home_path = std::env::var("HOME")
        .map_err(|_| AppError::Other("HOME environment variable not set".to_string()))?;
    let app_config_path = home_path + "/.config/shepherd/config.toml";
    let app_config = ShepherdConfig::from_file(&app_config_path).map_err(|e| {
        error!("Failed to load config: {}", e);
        AppError::Other("Failed to load config".to_string())
    });

    // if we have an error spit out the error and exit
    let app_config = match app_config {
        Ok(config) => config,
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };

    debug!("App config: {}", app_config);


    let auth_method = app_config.auth_method;
    let branch = app_config.branch;
    let cluster_ids = app_config.cluster_names.unwrap();
    let config_folder_path = app_config.rancher_config_path;
    let endpoint_url = app_config.endpoint_url;
    let file_format = app_config.file_format;
    let insecure = app_config.insecure;
    // in seconds
    let loop_interval = app_config.loop_interval;
    let remote_url = app_config.remote_git_url.unwrap();
    // in milliseconds
    let retry_delay = app_config.retry_delay;
    let token = app_config.token;
    
    let client = ShepherdClient::new(&endpoint_url, &token, insecure);
    let client_config = client.config.clone();

    run_sync(
        client_config,
        &config_folder_path,
        &remote_url,
        file_format,
        cluster_ids,
        loop_interval,
        retry_delay,
        &branch,
        auth_method,
    )
    .await?;

    Ok(())
}
