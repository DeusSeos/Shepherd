use std::{error::Error, path::{Path, PathBuf}};

use async_recursion::async_recursion;
use git2::{Commit, IndexAddOption, ProxyOptions, PushOptions, Repository, Signature, Status, StatusOptions};
use tokio::fs::read_dir;
use tracing::{debug, info, warn};

use crate::models::ObjectType;


/// Initialize a local git repository in the folder
///
/// # Arguments
/// 
/// * `folder_path` - Folder path to initialize the git repository  
/// 
/// # Returns
/// 
/// * `Result<(), String>` - Result indicating success or failure
///
pub fn init_and_commit_git_repo(folder_path: &Path, remote_url: &str) -> Result<(), String> {
    if !folder_path.exists() {
        warn!("Folder does not exist: {}", folder_path.display());
        return Err(format!("Folder does not exist: {}", folder_path.display()));
    }
    if !folder_path.is_dir() {
        warn!("Path is not a directory: {}", folder_path.display());
        return Err(format!("Path is not a directory: {}", folder_path.display()));
    }
    if folder_path.read_dir().map_err(|e| format!("Failed to read directory: {}", e))?.next().is_none() {
        warn!("Directory is empty: {}", folder_path.display());
        return Err(format!("Directory is empty: {}", folder_path.display()));
    }
    if Repository::discover(folder_path).is_ok() {
        warn!("Folder is already a git repository: {}", folder_path.display());
        return Err(format!("Folder is already a git repository: {}", folder_path.display()));
    }

    debug!("Initializing repository at {}", folder_path.display());
    let repo = Repository::init(folder_path).map_err(|e| format!("Failed to initialize repository: {}", e))?;

    debug!("Setting up remote at {}", remote_url);
    repo.remote("origin", remote_url).map_err(|e| format!("Failed to set up remote: {}", e))?;

    debug!("Adding all files to the index");
    let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None).map_err(|e| format!("Failed to add files to index: {}", e))?;
    index.write().map_err(|e| format!("Failed to write index: {}", e))?;

    debug!("Creating signature");
    let signature = repo.signature().or_else(|_| {
        Signature::now(crate::FULL_CLIENT_ID, "gitops@example.com")
    }).map_err(|e| format!("Failed to create signature: {}", e))?;

    debug!("Writing tree and creating commit");
    let tree_oid = index.write_tree().map_err(|e| format!("Failed to write tree: {}", e))?;
    let tree = repo.find_tree(tree_oid).map_err(|e| format!("Failed to find tree: {}", e))?;

    let commit_oid = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    ).map_err(|e| format!("Failed to create commit: {}", e))?;

    info!("Created commit with id: {}", commit_oid);
    Ok(())
}

/// Collect the modified files from a given folder path
///
/// # Arguments
/// * `folder_path` - The path of the folder to collect files from.
///
/// # Returns
/// A vector containing the absolute paths of all modified files
/// in the specified folder and its subfolders.
#[async_backtrace::framed]
pub async fn get_modified_files(
    folder_path: &Path,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let folder_path = folder_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize folder path {}: {}", folder_path.display(), e))?;
    let repo = Repository::discover(&folder_path)
        .map_err(|e| format!("Failed to open Git repo: {}", e))?;
    let workdir = repo
        .workdir()
        .ok_or("Repository has no working directory")?;

    debug!("Getting modified files from folder: {}", folder_path.display());
    debug!("Workdir is: {}", workdir.display());

    let rel_folder = folder_path
        .strip_prefix(workdir)
        .map_err(|_| format!("Folder path is not under workdir: {}", folder_path.display()))?;

    debug!("Computing relative folder path: rel_folder={:?}", rel_folder);
    let statuses = repo
        .statuses(None)
        .map_err(|e| format!("Failed to get statuses: {}", e))?;

    let mask = Status::WT_MODIFIED | Status::INDEX_MODIFIED;
    let mut modified_files = Vec::new();
    for status in statuses.iter() {

        if !status.status().intersects(mask) {
            debug!("Skipping file with status: {:?}", status.status());
            continue;
        }
        let path = match status.path() {
            Some(p) => std::path::Path::new(p),
            None => {
                warn!("Encountered null path in git status");
                continue;
            }
        };
        debug!("Processing path: {:?}", path);
        if path.starts_with(rel_folder) {
            debug!("Path is under rel_folder: {:?}", path);
            modified_files.push(workdir.join(path));
        }
    }
    debug!("Modified files: {:?}", modified_files);
    Ok(modified_files)
}


/// Initialize a local git repository in the folder with main branch
/// # Arguments
/// * `folder_path` - Folder path to initialize the git repository
/// * `remote_url` - Remote URL to set up
/// 
/// # Returns
/// * `Result<(), String>` - Result indicating success or failure
/// 
pub fn init_git_repo_with_main_branch(folder_path: &Path, remote_url: &str) -> Result<(), String> {
    if !folder_path.exists() {
        return Err(format!("Folder does not exist: {}", folder_path.display()));
    }
    if !folder_path.is_dir() {
        return Err(format!("Path is not a directory: {}", folder_path.display()));
    }
    if Repository::discover(folder_path).is_ok() {
        return Err(format!("Folder is already a git repository: {}", folder_path.display()));
    }

    debug!("Initializing repository in folder: {}", folder_path.display());
    let repo = Repository::init(folder_path).map_err(|e| format!("Failed to initialize repository: {}", e))?;

    // Set up remote
    debug!("Setting up remote: {}", remote_url);
    repo.remote("origin", remote_url).map_err(|e| format!("Failed to set up remote: {}", e))?;

    // Add all files
    debug!("Adding all files to the index");
    let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;
    index.add_all(["*"], IndexAddOption::FORCE, None).map_err(|e| format!("Failed to add files: {}", e))?;
    index.write().map_err(|e| format!("Failed to write index: {}", e))?;

    let tree_oid = index.write_tree().map_err(|e| format!("Failed to write tree: {}", e))?;
    debug!("Writing tree with OID: {}", tree_oid);
    let tree = repo.find_tree(tree_oid).map_err(|e| format!("Failed to find tree: {}", e))?;

    debug!("Creating signature");
    let sig = repo.signature().or_else(|_| {
        debug!("Failed to get signature, creating new one");
        Signature::now("GitOps Bot", "gitops@example.com")
    }).map_err(|e| format!("Failed to create signature: {}", e))?;

    // Create commit on orphan branch "main"
    let commit_oid = repo.commit(
        Some("refs/heads/main"),
        &sig,
        &sig,
        "Initial commit on main",
        &tree,
        &[],
    ).map_err(|e| format!("Failed to create commit: {}", e))?;

    // Update HEAD to point to "main"
    debug!("Updating HEAD to point to 'main'");
    repo.set_head("refs/heads/main").map_err(|e| format!("Failed to set HEAD: {}", e))?;

    info!("Initialized repository with main branch. Commit: {}", commit_oid);
    Ok(())
}


/// push repo to remote
/// # Arguments
/// * `folder_path` - Folder path to the git repository
/// * `remote_url` - Remote URL to push to
/// 
/// # Returns
/// * `Result<(), String>` - Result indicating success or failure
/// 
pub fn push_repo_to_remote(folder_path: &Path, remote_url: &str) -> Result<(), String> {
    debug!("Pushing repository at {} to remote: {}", folder_path.display(), remote_url);
    if !folder_path.exists() {
        return Err(format!("Folder does not exist: {}", folder_path.display()));
    }
    if !folder_path.is_dir() {
        return Err(format!("Path is not a directory: {}", folder_path.display()));
    }
    if Repository::discover(folder_path).is_ok() {
        return Err(format!("Folder is already a git repository: {}", folder_path.display()));
    }

    let repo = Repository::open(folder_path).map_err(|e| format!("Failed to open repository: {}", e))?;

    // Set up remote callbacks
    let mut remote_callbacks = git2::RemoteCallbacks::new();
    remote_callbacks
        .credentials(|_url, username_from_url, _allowed_types| {
            let username = username_from_url.unwrap_or("git");
            // TODO: Change this to use path fetched from our custom config file
            let private_key = Path::new("/Users/dc/.ssh/rancher_config");
            debug!("Using private key: {}", private_key.display());
            git2::Cred::ssh_key(username, None, private_key, None)
        })
        .transfer_progress(|progress| {
            debug!(
                "Transferred {} bytes out of {} bytes",
                progress.received_bytes(),
                progress.total_objects()
            );
            true
        })
        .update_tips(|refname, old_oid, new_oid| {
            debug!("Updated reference {} from {} to {}", refname, old_oid, new_oid);
            true
        });

    // Setup ProxyOptions
    let mut proxy_options = ProxyOptions::new();
    proxy_options.auto();

    // Setup PushOptions
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(remote_callbacks);
    push_options.proxy_options(proxy_options);

    // push to remote
    let mut remote = repo.find_remote("origin").or_else(|_| {
        repo.remote("origin", remote_url)
    }).map_err(|e| format!("Failed to find or create remote 'origin': {}", e))?;


    // Perform push
    remote.push(
        &["refs/heads/main:refs/heads/main"],
        Some(&mut push_options),
    ).map_err(|e| format!("Failed to push to remote: {}", e))?;
    debug!("Push completed successfully.");
    Ok(())
}

/// Commits changes in a given folder path with the specified commit message.
/// # Arguments
/// * `folder_path` - The path of the folder containing the changes.
/// * `message` - The commit message.
/// # Returns
/// * `Result<(), String>` - A result indicating success or failure.
pub fn commit_changes(folder_path: &Path, message: &str) -> Result<(), String> {
    if !folder_path.exists() {
        warn!("Folder does not exist: {}", folder_path.display());
        return Err(format!("Folder does not exist: {}", folder_path.display()));
    }
    if !folder_path.is_dir() {
        warn!("Path is not a directory: {}", folder_path.display());
        return Err(format!("Path is not a directory: {}", folder_path.display()));
    }

    debug!("Attempting to open repository at: {}", folder_path.display());
    let repo = Repository::open(folder_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    debug!("Getting repository index");
    let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;
    debug!("Adding all files to index");
    index.add_all(["*"], IndexAddOption::FORCE, None)
        .map_err(|e| format!("Failed to add files to index: {}", e))?;
    index.write().map_err(|e| format!("Failed to write index: {}", e))?;

    debug!("Writing tree from index");
    let tree_oid = index.write_tree().map_err(|e| format!("Failed to write tree: {}", e))?;
    let tree = repo.find_tree(tree_oid).map_err(|e| format!("Failed to find tree: {}", e))?;

    debug!("Creating commit signature");
    let sig = repo.signature().or_else(|_| {
        Signature::now(crate::FULL_CLIENT_ID, "shepherd@test.com")
    }).map_err(|e| format!("Failed to create signature: {}", e))?;

    debug!("Preparing parent commits");
    let parents: Vec<Commit> = match repo.head() {
        Ok(reference) => {
            if reference.is_branch() {
                let parent_commit = reference
                    .peel_to_commit()
                    .map_err(|e| format!("Failed to get parent commit: {}", e))?;
                vec![parent_commit]
            } else {
                vec![]
            }
        },
        Err(_) => vec![], // Initial commit
    };

    let parent_refs: Vec<&Commit> = parents.iter().collect();

    debug!("Creating commit");
    let commit_oid = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &parent_refs,
    ).map_err(|e| format!("Failed to create commit: {}", e))?;

    info!("Created commit with id: {}", commit_oid);
    Ok(())
}


/// Collect the new uncommitted (untracked) files from a given folder path
///
/// # Arguments
/// * `folder_path` - The path of the folder to collect files from.
///
/// # Returns
/// A vector containing the absolute paths of all uncommitted (untracked) files
/// in the specified folder and its subfolders.
#[async_backtrace::framed]
#[async_recursion]
pub async fn get_new_uncommited_files(
    folder_path: &Path,
) -> Result<Vec<(ObjectType, PathBuf)>, Box<dyn Error>> {
    let repo = Repository::discover(folder_path)
        .map_err(|e| format!("Failed to open Git repo: {}", e))?;
    let workdir = repo
        .workdir()
        .ok_or("Repository has no working directory")?;

    let mut new_files = Vec::new();
    let mut dir = read_dir(folder_path)
        .await
        .map_err(|e| format!("Failed to read dir {:?}: {}", folder_path, e))?;

    while let Some(entry) = dir.next_entry().await
        .map_err(|e| format!("Failed to read dir entry: {}", e))?
    {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        debug!("Processing: {:?}", path);

        if entry.file_type().await?.is_dir() && name == ".git" {
            debug!("Skipping .git directory");
            continue;
        }

        let metadata = entry
            .metadata()
            .await
            .map_err(|e| format!("Failed to stat {:?}: {}", path, e))?;

        if metadata.is_dir() {
            debug!("Directory: {:?}", path);
            let mut child = get_new_uncommited_files(&path).await?;
            new_files.append(&mut child);
        } else if metadata.is_file() {
            debug!("File: {:?}", path);
            let rel = path.strip_prefix(workdir)
                .map_err(|_| format!("File not under workdir: {:?}", path))?;

            let status = repo.status_file(rel)
                .map_err(|e| format!("Git status error for {:?}: {}", rel, e))?;

            if status.contains(Status::WT_NEW) {
                // Determine object type from path
                let object_type = determine_object_type(rel);
                debug!("New file: {:?}, type: {:?}", rel, object_type);
                new_files.push((object_type, path));
            }
        }
    }

    new_files.sort_by_key(|(object_type, _)| match object_type {
        ObjectType::RoleTemplate => 0,
        ObjectType::Project => 1,
        ObjectType::ProjectRoleTemplateBinding => 2,
        ObjectType::Cluster => 3, // optional: push clusters to the end
    });

    debug!("Collected new files: {:?}", new_files);

    Ok(new_files)
}



/// Collect the deleted files from a given folder path
///
/// # Arguments
/// * `folder_path` - The path of the folder to collect files from.
///
/// # Returns
/// A vector containing the absolute paths of all deleted files
/// in the specified folder and its subfolders, along with their corresponding
/// object type.

#[async_backtrace::framed]
#[async_recursion]
pub async fn get_deleted_files(
    folder_path: &Path,
) -> Result<Vec<(ObjectType, PathBuf)>, Box<dyn Error>> {
    debug!("Discovering repository in folder: {}", folder_path.display());
    let repo = Repository::discover(folder_path)
        .map_err(|e| format!("Failed to open Git repo: {}", e))?;
    let workdir = repo
        .workdir()
        .ok_or("Repository has no working directory")?;
    debug!("Repository workdir: {}", workdir.display());

    let mut deleted_files = Vec::new();

    let mut opts = StatusOptions::new();
    opts.include_untracked(false)
        .include_ignored(false)
        .recurse_untracked_dirs(true)
        .include_unmodified(false);

    debug!("Fetching statuses for deleted files");
    let statuses = repo.statuses(Some(&mut opts))?;

    for entry in statuses.iter() {
        let status = entry.status();
        let rel_path = match entry.path() {
            Some(p) => p,
            None => {
                warn!("Encountered entry with no path in statuses");
                continue;
            }
        };

        if status.contains(Status::WT_DELETED) {
            let full_path = workdir.join(rel_path);
            let object_type = determine_object_type(Path::new(rel_path));
            debug!("Deleted file: {:?}, type: {:?}", rel_path, object_type);
            deleted_files.push((object_type, full_path));
        } else {
            debug!("Skipping non-deleted file: {:?}", rel_path);
        }
    }

    debug!("Collected deleted files: {:?}", deleted_files);

    Ok(deleted_files)
}


/// Determine the object type from a path.
///
/// # Arguments
/// * `path` - The path to determine the object type from.
///
/// # Returns
/// The object type determined from the path.
fn determine_object_type(path: &Path) -> ObjectType {
    let file_name = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or_default();

    let file_extension = if let Some(ext) = path.extension() {
        ext.to_string_lossy().to_lowercase()
    } else {
        String::new()
    };

    match (
        file_name.ends_with(&format!(".project.{}", file_extension)),
        file_name.ends_with(&format!(".prtb.{}", file_extension)),
        file_name.ends_with(&format!(".rt.{}", file_extension)),
        file_name.ends_with(&format!(".cluster.{}", file_extension)),
    ) {
        (true, _, _, _) => ObjectType::Project,
        (_, true, _, _) => ObjectType::ProjectRoleTemplateBinding,
        (_, _, true, _) => ObjectType::RoleTemplate,
        (_, _, _, true) => ObjectType::Cluster,
        _ => {
            if path.components().any(|c| c.as_os_str() == "roles") {
                ObjectType::RoleTemplate
            } else if file_name.starts_with("prtb-") {
                ObjectType::ProjectRoleTemplateBinding
            } else {
                ObjectType::Project
            }
        }
    }
}

/// Collects deleted files and their contents from a given folder path.

/// # Arguments
/// * `folder_path` - The path of the folder to collect deleted files from.
///
/// # Returns
/// A vector of tuples containing the object type, absolute path, and contents
/// of each deleted file.
#[async_backtrace::framed]
pub async fn get_deleted_files_and_contents(
    folder_path: &Path,
) -> Result<Vec<(ObjectType, PathBuf, String)>, Box<dyn Error>> {
    // Discover the Git repository at the given folder path
    let repo = Repository::discover(folder_path)
        .map_err(|e| format!("Failed to open Git repo: {}", e))?;
    let workdir = repo
        .workdir()
        .ok_or("Repository has no working directory")?;

    debug!("Collecting deleted files...");
    let mut deleted_files = Vec::new();

    // Configure status options to exclude untracked and ignored files
    let mut opts = StatusOptions::new();
    opts.include_untracked(false)
        .include_ignored(false)
        .include_unmodified(false)
        .recurse_untracked_dirs(true);

    // Collect the statuses of files in the repository
    let statuses = repo.statuses(Some(&mut opts))?;
    debug!("Collected statuses");

    // Retrieve the HEAD commit and its associated tree
    let head_commit = repo.revparse_single("HEAD^{commit}")?;
    debug!("Got HEAD commit");
    let tree = head_commit.peel_to_tree()?;
    debug!("Got tree");

    // Iterate through each entry in the statuses
    for entry in statuses.iter() {
        let status = entry.status();
        let rel_path = match entry.path() {
            Some(p) => p,
            None => {
                warn!("Encountered entry with no path in statuses");
                continue;
            }
        };

        // Check if the file is marked as deleted
        if status.contains(Status::WT_DELETED) {
            let full_path = workdir.join(rel_path);
            let git_rel_path = Path::new(rel_path);

            debug!("Attempting to get blob from HEAD for deleted file: {:?}", git_rel_path);
            // Attempt to retrieve blob from the HEAD commit
            match tree.get_path(git_rel_path) {
                Ok(tree_entry) => {
                    let object = tree_entry.to_object(&repo)?;
                    let blob = object.peel_to_blob()?;
                    let contents = String::from_utf8(blob.content().to_vec())
                        .map_err(|e| format!("Invalid UTF-8 in blob: {}", e))?;

                    // Determine the object type from the path
                    let object_type = determine_object_type(git_rel_path);
                    debug!("Determined object type {:?} for deleted file {:?}", object_type, git_rel_path);
                    deleted_files.push((object_type, full_path, contents));
                }
                Err(e) => {
                    warn!("Unable to retrieve blob from HEAD for deleted file: {}", e);
                }
            }
        }
    }

    Ok(deleted_files)
}
