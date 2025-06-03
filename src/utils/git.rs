use std::{
    error::Error,
    path::{Path, PathBuf},
    time::Duration,
};

use async_recursion::async_recursion;
use git2::{
    Commit, Error as Git2Error, ErrorCode, Index, IndexAddOption, Oid, ProxyOptions, PushOptions,
    RemoteCallbacks, Repository, Signature, Status, StatusOptions,
};

use serde::{Deserialize, Serialize};
use tokio::{fs::read_dir, time::sleep};
use tracing::{debug, error, info, warn};

use crate::models::ObjectType;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GitAuth {
    SshKey(PathBuf),
    HttpsToken(String),
    SshAgent,
    GitCredentialHelper,
}

use thiserror::Error;

use super::file::is_directory_empty;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Git error: {0}")]
    Git(#[from] Git2Error),
    #[error("Directory is empty: {0}")]
    EmptyDirectory(String),
    #[error("Directory already contains a git repository: {0}")]
    ExistingRepository(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Other error: {0}")]
    Other(String),
}

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(5);

pub async fn init_and_commit_git_repo(
    folder_path: &Path,
    remote_url: &str,
) -> Result<(), GitError> {
    let mut retries = 0;
    loop {
        match init_and_commit_git_repo_inner(folder_path, remote_url).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                if retries >= MAX_RETRIES {
                    return Err(e);
                }
                match e {
                    GitError::Network(_) => {
                        warn!(
                            "Network error occurred, retrying in {} seconds",
                            RETRY_DELAY.as_secs()
                        );
                        sleep(RETRY_DELAY).await;
                        retries += 1;
                    }
                    _ => return Err(e),
                }
            }
        }
    }
}

pub async fn safe_clone_repository(
    config_folder_path: &Path,
    remote_url: &str,
    auth_method: &GitAuth,
) -> Result<Repository, GitError> {
    if is_directory_empty(config_folder_path)
        .await
        .map_err(|e| GitError::Other(format!("Failed to check if directory is empty: {}", e)))?
    {
        // Directory is empty, attempt to clone the repository
        info!("Cloning repository from {}", remote_url);

        let mut fetch_options = git2::FetchOptions::new();

        // Set up remote callbacks
        let mut remote_callbacks = git2::RemoteCallbacks::new();
        remote_callbacks
            .credentials(|_url, username_from_url, allowed_types| {
                match &auth_method {
                    GitAuth::SshKey(key_path) => {
                        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                            let username = username_from_url.unwrap_or("git");
                            git2::Cred::ssh_key(username, None, key_path, None)
                        } else {
                            Err(git2::Error::from_str("SSH key authentication not allowed"))
                        }
                    }
                    GitAuth::HttpsToken(token) => {
                        if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                            git2::Cred::userpass_plaintext(username_from_url.unwrap_or(""), token)
                        } else {
                            Err(git2::Error::from_str(
                                "HTTPS token authentication not allowed",
                            ))
                        }
                    }
                    GitAuth::SshAgent => {
                        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                            git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
                        } else {
                            Err(git2::Error::from_str(
                                "SSH agent authentication not allowed",
                            ))
                        }
                    }
                    _ => Err(git2::Error::from_str("Unsupported authentication method")),
                }
                .map_err(|e| e.into())
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
                debug!(
                    "Updated reference {} from {} to {}",
                    refname, old_oid, new_oid
                );
                true
            });

        fetch_options.remote_callbacks(remote_callbacks);

        // Use RepoBuilder to clone the repository
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        match builder.clone(remote_url, config_folder_path) {
            Ok(repo) => Ok(repo),
            Err(e) => {
                // Check if the error is related to an empty repository
                if e.message()
                    .contains("remote HEAD refers to nonexistent ref")
                    || e.message().contains("unable to checkout")
                {
                    info!("Remote repository appears to be empty. Initializing local repository instead.");

                    // Initialize a new repository locally
                    let repo = Repository::init(config_folder_path)?;

                    // Set up the remote
                    repo.remote("origin", remote_url)?;

                    Ok(repo)
                } else {
                    // For other errors, propagate them
                    Err(GitError::Git(e))
                }
            }
        }
    } else {
        // If the directory is not empty, try to open the repository
        match Repository::open(config_folder_path) {
            Ok(repo) => Ok(repo),
            Err(_) => Err(GitError::ExistingRepository(
                config_folder_path.display().to_string(),
            )),
        }
    }
}

async fn init_and_commit_git_repo_inner(
    folder_path: &Path,
    remote_url: &str,
) -> Result<(), GitError> {
    if !folder_path.exists() {
        return Err(GitError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Folder does not exist",
        )));
    }
    if !folder_path.is_dir() {
        return Err(GitError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Path is not a directory",
        )));
    }
    if folder_path.read_dir()?.next().is_none() {
        return Err(GitError::EmptyDirectory(folder_path.display().to_string()));
    }
    if Repository::discover(folder_path).is_ok() {
        return Err(GitError::ExistingRepository(
            folder_path.display().to_string(),
        ));
    }

    let repo = Repository::init(folder_path).map_err(GitError::Git)?;

    repo.remote("origin", remote_url).map_err(GitError::Git)?;

    let mut index = repo.index().map_err(GitError::Git)?;
    index
        .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .map_err(GitError::Git)?;
    index.write().map_err(GitError::Git)?;

    let tree_id = index.write_tree().map_err(GitError::Git)?;
    let tree = repo.find_tree(tree_id).map_err(GitError::Git)?;

    let signature = repo.signature().map_err(GitError::Git)?;
    let message = "Initial commit";
    let parents = &[];

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        parents,
    )
    .map_err(GitError::Git)?;

    // Push to remote
    let mut remote = repo.find_remote("origin").map_err(GitError::Git)?;
    let mut callbacks = RemoteCallbacks::new();
    callbacks.push_update_reference(|refname, status| {
        if let Some(msg) = status {
            Err(Git2Error::from_str(&format!(
                "Failed to update {}: {}",
                refname, msg
            )))
        } else {
            Ok(())
        }
    });

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    remote
        .push(
            &["refs/heads/main:refs/heads/main"],
            Some(&mut push_options),
        )
        .map_err(|e| GitError::Network(format!("Failed to push: {}", e)))?;

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
pub async fn get_modified_files(folder_path: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let folder_path = folder_path.canonicalize().map_err(|e| {
        format!(
            "Failed to canonicalize folder path {}: {}",
            folder_path.display(),
            e
        )
    })?;
    let repo = Repository::discover(&folder_path)
        .map_err(|e| format!("Failed to open Git repo: {}", e))?;
    let workdir = repo
        .workdir()
        .ok_or("Repository has no working directory")?;

    debug!(
        "Getting modified files from folder: {}",
        folder_path.display()
    );
    debug!("Workdir is: {}", workdir.display());

    let rel_folder = folder_path.strip_prefix(workdir).map_err(|_| {
        format!(
            "Folder path is not under workdir: {}",
            folder_path.display()
        )
    })?;

    debug!(
        "Computing relative folder path: rel_folder={:?}",
        rel_folder
    );
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
pub fn init_git_repo_with_main_branch(
    folder_path: &Path,
    remote_url: &str,
    branch_name: &str,
) -> Result<(), GitError> {
    if !folder_path.exists() {
        return Err(GitError::Other(format!(
            "Folder does not exist: {}",
            folder_path.display()
        )));
    }
    if !folder_path.is_dir() {
        return Err(GitError::Other(format!(
            "Path is not a directory: {}",
            folder_path.display()
        )));
    }

    debug!(
        "Initializing repository in folder: {}",
        folder_path.display()
    );
    let repo = Repository::init(folder_path).map_err(GitError::Git)?;

    debug!("Creating an initial commit in repository");
    let sig = Signature::now(crate::FULL_CLIENT_ID, "shepherd@test.com").map_err(GitError::Git)?;
    let tree_id = {
        let mut index = repo.index().map_err(GitError::Git)?;
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(GitError::Git)?;
        index.write_tree().map_err(GitError::Git)?
    };
    let tree = repo.find_tree(tree_id).map_err(GitError::Git)?;
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .map_err(GitError::Git)?;

    if repo.find_remote("origin").is_ok() {
        debug!("Remote 'origin' already exists — updating URL if necessary");

        let current_url = repo
            .find_remote("origin")?
            .url()
            .unwrap_or("<none>")
            .to_string();

        if current_url != remote_url {
            debug!(
                "Updating remote 'origin' URL from {} to {}",
                current_url, remote_url
            );
            repo.remote_set_url("origin", remote_url)?;
        }
    } else {
        debug!("Creating new remote 'origin' -> {}", remote_url);
        repo.remote("origin", remote_url)?;
    }

    debug!("Creating and checking out branch: {}", branch_name);
    let mut branch = repo
        .branch(branch_name, &repo.head()?.peel_to_commit()?, false)
        .map_err(GitError::Git)?;
    repo.set_head(branch.get().name().unwrap_or("refs/heads/master"))
        .map_err(GitError::Git)?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().safe()))
        .map_err(GitError::Git)?;

    // Set up branch to track remote
    branch
        .set_upstream(Some("origin/main"))
        .map_err(GitError::Git)?;

    Ok(())
}

pub fn resolve_conflicts(repo: &Repository, branch: &str) -> Result<(), GitError> {
    let mut index = repo.index()?;
    if index.has_conflicts() {
        warn!("Merge conflicts detected. Attempting to resolve...");

        // Resolve conflicts by taking 'ours'
        resolve_index_conflicts(&mut index)?;

        // Write the updated index to disk
        index.write()?;

        // Create the merge commit
        create_merge_commit(repo, &mut index, branch)?;

        info!("Conflicts resolved and merge commit created");
    }
    Ok(())
}

fn resolve_index_conflicts(index: &mut Index) -> Result<(), GitError> {
    // Collect all conflicts to avoid borrowing the index during iteration
    let conflicts = index.conflicts()?.collect::<Result<Vec<_>, _>>()?;

    for conflict in conflicts {
        // Favor "theirs", fallback to "ours"
        let chosen_entry = conflict.their.or(conflict.our);
        if let Some(entry) = chosen_entry {
            index.add(&entry)?;

            // Resolve the path to remove the conflict
            let path = Path::new(
                std::str::from_utf8(&entry.path)
                    .map_err(|e| GitError::Other(format!("Invalid UTF-8 in path: {}", e)))?,
            );
            index.remove_path(path)?;
        } else {
            // No "theirs" or "ours" — fall back to ancestor or skip
            if let Some(ancestor) = conflict.ancestor {
                let path = Path::new(std::str::from_utf8(&ancestor.path).map_err(|e| {
                    GitError::Other(format!("Invalid UTF-8 in ancestor path: {}", e))
                })?);
                index.remove_path(path)?;
            }
        }
    }

    Ok(())
}

fn create_merge_commit(
    repo: &Repository,
    index: &mut Index,
    branch: &str,
) -> Result<Oid, GitError> {
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let signature = repo.signature()?;
    let parent_commit = repo.head()?.peel_to_commit()?;
    let message = "Merge and resolve conflicts";

    // Get the FETCH_HEAD as the second parent
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = fetch_head.peel_to_commit()?;

    let commit_id = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent_commit, &fetch_commit],
    )?;

    // Update the branch reference
    let refname = format!("refs/heads/{}", branch);
    repo.reference(&refname, commit_id, true, "merge: Fast-forward")?;

    Ok(commit_id)
}

/// push repo to remote
/// # Arguments
/// * `folder_path` - Folder path to the git repository
/// * `remote_url` - Remote URL to push to
///
/// # Returns
/// * `Result<(), String>` - Result indicating success or failure
///
pub fn push_repo_to_remote(
    folder_path: &Path,
    remote_url: &str,
    auth_method: GitAuth,
) -> Result<(), String> {
    debug!(
        "Pushing repository at {} to remote: {}",
        folder_path.display(),
        remote_url
    );
    if !folder_path.exists() {
        return Err(format!("Folder does not exist: {}", folder_path.display()));
    }
    if !folder_path.is_dir() {
        return Err(format!(
            "Path is not a directory: {}",
            folder_path.display()
        ));
    }

    let repo =
        Repository::open(folder_path).map_err(|e| format!("Failed to open repository: {}", e))?;
    let config = repo
        .config()
        .map_err(|e| format!("Failed to get repository config: {}", e))?;

    // Set up remote callbacks
    let mut remote_callbacks = git2::RemoteCallbacks::new();
    remote_callbacks
        .credentials(|url, username_from_url, allowed_types| {
            match &auth_method {
                GitAuth::SshKey(key_path) => {
                    if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                        let username = username_from_url.unwrap_or("git");
                        git2::Cred::ssh_key(username, None, key_path, None)
                    } else {
                        Err(git2::Error::from_str("SSH key authentication not allowed"))
                    }
                }
                GitAuth::HttpsToken(token) => {
                    if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                        git2::Cred::userpass_plaintext(username_from_url.unwrap_or(""), token)
                    } else {
                        Err(git2::Error::from_str(
                            "HTTPS token authentication not allowed",
                        ))
                    }
                }
                GitAuth::SshAgent => {
                    if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                        debug!("Using SSH agent authentication");
                        debug!("Auth method: {:#?}", auth_method);
                        git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
                    } else {
                        Err(git2::Error::from_str(
                            "SSH agent authentication not allowed",
                        ))
                    }
                }
                GitAuth::GitCredentialHelper => {
                    if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                        git2::Cred::credential_helper(&config, url, username_from_url)
                    } else {
                        Err(git2::Error::from_str(
                            "Git credential helper authentication not allowed",
                        ))
                    }
                }
            }
            .map_err(|e| e.into())
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
            debug!(
                "Updated reference {} from {} to {}",
                refname, old_oid, new_oid
            );
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
    let mut remote = repo
        .find_remote("origin")
        .or_else(|_| repo.remote("origin", remote_url))
        .map_err(|e| format!("Failed to find or create remote 'origin': {}", e))?;

    // Perform push
    remote
        .push(
            &["refs/heads/main:refs/heads/main"],
            Some(&mut push_options),
        )
        .map_err(|e| format!("Failed to push to remote: {}", e))?;
    debug!("Push completed successfully.");
    Ok(())
}

pub fn pull_changes(
    repo: &Repository,
    branch: &str,
    auth_method: &GitAuth,
) -> Result<(), GitError> {
    // Setup ProxyOptions
    let mut proxy_options = ProxyOptions::new();
    proxy_options.auto();

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, allowed_types| {
        match_credentials(url, username_from_url, allowed_types, auth_method).map_err(|e| e.into())
    });

    let mut remote = repo.find_remote("origin")?;
    remote.connect_auth(git2::Direction::Fetch, Some(callbacks), Some(proxy_options))?;

    let mut fetch_options = git2::FetchOptions::new();

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, allowed_types| {
        match_credentials(url, username_from_url, allowed_types, auth_method).map_err(|e| e.into())
    });

    let mut proxy_options = ProxyOptions::new();
    proxy_options.auto();

    fetch_options.remote_callbacks(callbacks);
    fetch_options.proxy_options(proxy_options);

    remote.fetch(&[branch], Some(&mut fetch_options), None)?;

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.0.is_up_to_date() {
        Ok(())
    } else if analysis.0.is_fast_forward() {
        let refname = format!("refs/heads/{}", branch);
        let mut reference = repo.find_reference(&refname)?;
        reference.set_target(fetch_commit.id(), "Fast-Forward")?;
        repo.set_head(&refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        Ok(())
    } else {
        Err(GitError::Other("Merge analysis failed".to_string()))
    }
}

fn match_credentials(
    url: &str,
    username_from_url: Option<&str>,
    allowed_types: git2::CredentialType,
    auth_method: &GitAuth,
) -> Result<git2::Cred, git2::Error> {
    match &auth_method {
        GitAuth::SshKey(key_path) => {
            if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                let username = username_from_url.unwrap_or("git");
                debug!("SSH Key using Username: {}", username);
                git2::Cred::ssh_key(username, None, key_path, None)
            } else {
                Err(git2::Error::from_str("SSH key authentication not allowed"))
            }
        }
        GitAuth::HttpsToken(token) => {
            if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                git2::Cred::userpass_plaintext(username_from_url.unwrap_or(""), token)
            } else {
                Err(git2::Error::from_str(
                    "HTTPS token authentication not allowed",
                ))
            }
        }
        GitAuth::SshAgent => {
            if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
            } else {
                Err(git2::Error::from_str(
                    "SSH agent authentication not allowed",
                ))
            }
        }
        _ => Err(git2::Error::from_str("Unsupported authentication method")),
    }
}

pub fn push_changes(
    repo: &Repository,
    branch: &str,
    auth_method: &GitAuth,
) -> Result<(), GitError> {
    let mut remote_callbacks = RemoteCallbacks::new();
    remote_callbacks.credentials(|url, username_from_url, allowed_types| {
        match_credentials(url, username_from_url, allowed_types, auth_method).map_err(|e| e.into())
    });

    let mut proxy_options = ProxyOptions::new();
    proxy_options.auto();

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(remote_callbacks);
    push_options.proxy_options(proxy_options);

    let mut remote = repo.find_remote("origin")?;
    let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);
    remote.push(&[&refspec], Some(&mut push_options))?;
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
        return Err(format!(
            "Path is not a directory: {}",
            folder_path.display()
        ));
    }

    debug!(
        "Attempting to open repository at: {}",
        folder_path.display()
    );
    let repo =
        Repository::open(folder_path).map_err(|e| format!("Failed to open repository: {}", e))?;

    debug!("Getting repository index");
    let mut index = repo
        .index()
        .map_err(|e| format!("Failed to get index: {}", e))?;
    debug!("Adding all files to index");
    index
        .add_all(["*"], IndexAddOption::FORCE, None)
        .map_err(|e| format!("Failed to add files to index: {}", e))?;
    index
        .write()
        .map_err(|e| format!("Failed to write index: {}", e))?;

    debug!("Writing tree from index");
    let tree_oid = index
        .write_tree()
        .map_err(|e| format!("Failed to write tree: {}", e))?;
    let tree = repo
        .find_tree(tree_oid)
        .map_err(|e| format!("Failed to find tree: {}", e))?;

    debug!("Creating commit signature");
    let sig = repo
        .signature()
        .or_else(|_| Signature::now(crate::FULL_CLIENT_ID, "shepherd@test.com"))
        .map_err(|e| format!("Failed to create signature: {}", e))?;

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
        }
        Err(_) => vec![], // Initial commit
    };

    let parent_refs: Vec<&Commit> = parents.iter().collect();

    debug!("Creating commit");
    let commit_oid = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
        .map_err(|e| format!("Failed to create commit: {}", e))?;

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
    let repo =
        Repository::discover(folder_path).map_err(|e| format!("Failed to open Git repo: {}", e))?;
    let workdir = repo
        .workdir()
        .ok_or("Repository has no working directory")?;

    let mut new_files = Vec::new();
    let mut dir = read_dir(folder_path)
        .await
        .map_err(|e| format!("Failed to read dir {:?}: {}", folder_path, e))?;

    while let Some(entry) = dir
        .next_entry()
        .await
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
            let rel = path
                .strip_prefix(workdir)
                .map_err(|_| format!("File not under workdir: {:?}", path))?;

            let status = repo
                .status_file(rel)
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
    debug!(
        "Discovering repository in folder: {}",
        folder_path.display()
    );
    let repo =
        Repository::discover(folder_path).map_err(|e| format!("Failed to open Git repo: {}", e))?;
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
    let repo =
        Repository::discover(folder_path).map_err(|e| format!("Failed to open Git repo: {}", e))?;
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

            debug!(
                "Attempting to get blob from HEAD for deleted file: {:?}",
                git_rel_path
            );
            // Attempt to retrieve blob from the HEAD commit
            match tree.get_path(git_rel_path) {
                Ok(tree_entry) => {
                    let object = tree_entry.to_object(&repo)?;
                    let blob = object.peel_to_blob()?;
                    let contents = String::from_utf8(blob.content().to_vec())
                        .map_err(|e| format!("Invalid UTF-8 in blob: {}", e))?;

                    // Determine the object type from the path
                    let object_type = determine_object_type(git_rel_path);
                    debug!(
                        "Determined object type {:?} for deleted file {:?}",
                        object_type, git_rel_path
                    );
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
