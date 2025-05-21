use std::{error::Error, path::{Path, PathBuf}};

use async_recursion::async_recursion;
use git2::{Commit, IndexAddOption, ProxyOptions, PushOptions, Repository, Signature, Status, StatusOptions};
use tokio::fs::read_dir;
use tracing::{debug, warn};

use crate::{file::{file_extension_from_format, FileFormat}, models::ObjectType};


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
        return Err(format!("Folder does not exist: {}", folder_path.display()));
    }
    if !folder_path.is_dir() {
        return Err(format!("Path is not a directory: {}", folder_path.display()));
    }
    if folder_path.read_dir().map_err(|e| format!("Failed to read directory: {}", e))?.next().is_none() {
        return Err(format!("Directory is empty: {}", folder_path.display()));
    }
    if Repository::discover(folder_path).is_ok() {
        return Err(format!("Folder is already a git repository: {}", folder_path.display()));
    }

    let repo = Repository::init(folder_path).map_err(|e| format!("Failed to initialize repository: {}", e))?;

    
    repo.remote("origin", remote_url).map_err(|e| format!("Failed to set up remote: {}", e))?;

    // init the git repository with main branch

    let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None).map_err(|e| format!("Failed to add files to index: {}", e))?;
    index.write().map_err(|e| format!("Failed to write index: {}", e))?; // <--- critical!

    let signature = repo.signature().or_else(|_| {
        Signature::now(crate::FULL_CLIENT_ID, "gitops@example.com")
    }).map_err(|e| format!("Failed to create signature: {}", e))?;
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

    println!("Created commit with id: {}", commit_oid);
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

    let rel_folder = folder_path
        .strip_prefix(workdir)
        .map_err(|_| format!("Folder path is not under workdir: {}", folder_path.display()))?;

    let statuses = repo
        .statuses(None)
        .map_err(|e| format!("Failed to get statuses: {}", e))?;
    let mask = Status::WT_MODIFIED | Status::INDEX_MODIFIED;
    let mut modified_files = Vec::new();
    for status in statuses.iter() {
        if !status.status().intersects(mask) {
            continue;
        }
        let path = match status.path() {
            Some(p) => std::path::Path::new(p),
            None => {
                tracing::warn!("Encountered null path in git status");
                continue;
            }
        };
        if path.starts_with(rel_folder) {
            modified_files.push(workdir.join(path));
        }
    }
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

    let repo = Repository::init(folder_path).map_err(|e| format!("Failed to initialize repository: {}", e))?;

    // Set up remote
    repo.remote("origin", remote_url).map_err(|e| format!("Failed to set up remote: {}", e))?;

    // Add all files
    let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;
    index.add_all(["*"], IndexAddOption::FORCE, None).map_err(|e| format!("Failed to add files {}", e))?;
    index.write().map_err(|e| format!("Failed to write index: {}", e))?;

    let tree_oid = index.write_tree().map_err(|e| format!("Failed to write tree: {}", e))?;
    let tree = repo.find_tree(tree_oid).map_err(|e| format!("Failed to find tree: {}", e))?;

    let sig = repo.signature().or_else(|_| {
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
    repo.set_head("refs/heads/main").map_err(|e| format!("Failed to set HEAD: {}", e))?;

    println!("Initialized repository with main branch. Commit: {}", commit_oid);
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
            let private_key = Path::new("/Users/dc/.ssh/rancher_config");
            git2::Cred::ssh_key(username, None, private_key, None)
        })
        .transfer_progress(|progress| {
            println!(
                "Transferred {} bytes out of {} bytes",
                progress.received_bytes(),
                progress.total_objects()
            );
            true
        })
        .update_tips(|refname, old_oid, new_oid| {
            println!("Updated reference {} from {} to {}", refname, old_oid, new_oid);
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
    println!("Push completed successfully.");
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
        return Err(format!("Folder does not exist: {}", folder_path.display()));
    }
    if !folder_path.is_dir() {
        return Err(format!("Path is not a directory: {}", folder_path.display()));
    }

    let repo = Repository::open(folder_path).map_err(|e| format!("Failed to open repository: {}", e))?;

    let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;
    index.add_all(["*"], IndexAddOption::FORCE, None).map_err(|e| format!("Failed to add files to index: {}", e))?;
    index.write().map_err(|e| format!("Failed to write index: {}", e))?;

    let tree_oid = index.write_tree().map_err(|e| format!("Failed to write tree: {}", e))?;
    let tree = repo.find_tree(tree_oid).map_err(|e| format!("Failed to find tree: {}", e))?;

    let sig = repo.signature().or_else(|_| {
        Signature::now(crate::FULL_CLIENT_ID, "shepherd@test.com")
    }).map_err(|e| format!("Failed to create signature: {}", e))?;

    // Own the parent commit so it lives long enough
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

    // Create a slice of references to the parents
    let parent_refs: Vec<&Commit> = parents.iter().collect();

    let commit_oid = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &parent_refs,
    ).map_err(|e| format!("Failed to create commit: {}", e))?;

    println!("Created commit with id: {}", commit_oid);
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

        if entry.file_type().await?.is_dir() && name == ".git" {
            continue;
        }

        let metadata = entry
            .metadata()
            .await
            .map_err(|e| format!("Failed to stat {:?}: {}", path, e))?;

        if metadata.is_dir() {
            let mut child = get_new_uncommited_files(&path).await?;
            new_files.append(&mut child);
        } else if metadata.is_file() {
            let rel = path.strip_prefix(workdir)
                .map_err(|_| format!("File not under workdir: {:?}", path))?;

            let status = repo.status_file(rel)
                .map_err(|e| format!("Git status error for {:?}: {}", rel, e))?;

            if status.contains(Status::WT_NEW) {
                // Determine object type from path
                let object_type = determine_object_type(rel);
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
    let repo = Repository::discover(folder_path)
        .map_err(|e| format!("Failed to open Git repo: {}", e))?;
    let workdir = repo
        .workdir()
        .ok_or("Repository has no working directory")?;

    let mut deleted_files = Vec::new();

    let mut opts = StatusOptions::new();
    opts.include_untracked(false)
        .include_ignored(false)
        .recurse_untracked_dirs(true)
        .include_unmodified(false);

    let statuses = repo.statuses(Some(&mut opts))?;

    for entry in statuses.iter() {
        let status = entry.status();
        let rel_path = match entry.path() {
            Some(p) => p,
            None => continue,
        };

        if status.contains(Status::WT_DELETED) {
            let full_path = workdir.join(rel_path);
            let object_type = determine_object_type(Path::new(rel_path));
            deleted_files.push((object_type, full_path));
        }
    }

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
    let file_name = path.file_name()
        .and_then(|f| f.to_str())
        .unwrap_or_default();

    if file_name.ends_with(&format!(".project.{}", file_extension_from_format(&FileFormat::Yaml))) || 
       file_name.ends_with(&format!(".project.{}", file_extension_from_format(&FileFormat::Json))) || 
       file_name.ends_with(&format!(".project.{}", file_extension_from_format(&FileFormat::Toml))) {
        ObjectType::Project
    } else if file_name.ends_with(&format!(".prtb.{}", file_extension_from_format(&FileFormat::Yaml))) || 
              file_name.ends_with(&format!(".prtb.{}", file_extension_from_format(&FileFormat::Json))) || 
              file_name.ends_with(&format!(".prtb.{}", file_extension_from_format(&FileFormat::Toml))) {
        ObjectType::ProjectRoleTemplateBinding
    } else if file_name.ends_with(&format!(".rt.{}", file_extension_from_format(&FileFormat::Yaml))) || 
              file_name.ends_with(&format!(".rt.{}", file_extension_from_format(&FileFormat::Json))) || 
              file_name.ends_with(&format!(".rt.{}", file_extension_from_format(&FileFormat::Toml))) {
        ObjectType::RoleTemplate
    } else if file_name.ends_with(&format!(".cluster.{}", file_extension_from_format(&FileFormat::Yaml))) || 
              file_name.ends_with(&format!(".cluster.{}", file_extension_from_format(&FileFormat::Json))) || 
              file_name.ends_with(&format!(".cluster.{}", file_extension_from_format(&FileFormat::Toml))) {
        ObjectType::Cluster
    } else {
        // Legacy support or default
        debug!("Could not determine object type from filename: {}", file_name);
        if path.components().any(|c| c.as_os_str() == "roles") {
            ObjectType::RoleTemplate
        } else if file_name.starts_with("prtb-") {
            ObjectType::ProjectRoleTemplateBinding
        } else {
            // Default to Project
            ObjectType::Project
        }
    }
}

#[async_backtrace::framed]
pub async fn get_deleted_files_and_contents(
    folder_path: &Path,
) -> Result<Vec<(ObjectType, PathBuf, String)>, Box<dyn Error>> {
    let repo = Repository::discover(folder_path)
        .map_err(|e| format!("Failed to open Git repo: {}", e))?;
    let workdir = repo
        .workdir()
        .ok_or("Repository has no working directory")?;

    let mut deleted_files = Vec::new();

    let mut opts = StatusOptions::new();
    opts.include_untracked(false)
        .include_ignored(false)
        .include_unmodified(false)
        .recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    let head_commit = repo.revparse_single("HEAD^{commit}")?;
    let tree = head_commit.peel_to_tree()?;

    for entry in statuses.iter() {
        let status = entry.status();
        let rel_path = match entry.path() {
            Some(p) => p,
            None => continue,
        };

        if status.contains(Status::WT_DELETED) {
            let full_path = workdir.join(rel_path);
            let git_rel_path = Path::new(rel_path);

            // Attempt to retrieve blob from HEAD commit
            match tree.get_path(git_rel_path) {
                Ok(tree_entry) => {
                    let object = tree_entry.to_object(&repo)?;
                    let blob = object.peel_to_blob()?;
                    let contents = String::from_utf8(blob.content().to_vec())
                        .map_err(|e| format!("Invalid UTF-8 in blob: {}", e))?;

                    // Use our updated determine_object_type function
                    let object_type = determine_object_type(git_rel_path);
                    debug!("Determined object type {:?} for deleted file {:?}", object_type, git_rel_path);
                    deleted_files.push((object_type, full_path, contents));
                }
                Err(e) => {
                    warn!(?rel_path, "Unable to retrieve blob from HEAD: {}", e);
                }
            }
        }
    }

    Ok(deleted_files)
}