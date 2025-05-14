use std::path::Path;

use git2::{Commit, IndexAddOption, ProxyOptions, PushOptions, Repository, Signature};


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


// /// Commit changes to the local git repository
// /// # Arguments
// /// * `folder_path` - Folder path to the git repository
// /// * `message` - Commit message
// /// 
// /// # Returns
// /// * `Result<(), String>` - Result indicating success or failure
// /// 
// pub fn commit_changes(folder_path: &Path, message: &str) -> Result<(), String> {
//     if !folder_path.exists() {
//         return Err(format!("Folder does not exist: {}", folder_path.display()));
//     }
//     if !folder_path.is_dir() {
//         return Err(format!("Path is not a directory: {}", folder_path.display()));
//     }

//     let repo = Repository::open(folder_path).map_err(|e| format!("Failed to open repository: {}", e))?;

//     let mut index = repo.index().map_err(|e| format!("Failed to get index: {}", e))?;
//     index.add_all(["*"], IndexAddOption::FORCE, None).map_err(|e| format!("Failed to add files to index: {}", e))?;
//     index.write().map_err(|e| format!("Failed to write index: {}", e))?;

//     let tree_oid = index.write_tree().map_err(|e| format!("Failed to write tree: {}", e))?;
//     let tree = repo.find_tree(tree_oid).map_err(|e| format!("Failed to find tree: {}", e))?;

//     let sig = repo.signature().or_else(|_| {
//         Signature::now("GitOps Bot", "gitops@example.com")
//     }).map_err(|e| format!("Failed to create signature: {}", e))?;
//     let commit_oid = repo.commit(
//         Some("HEAD"),
//         &sig,
//         &sig,
//         message,
//         &tree,
//         &[],
//     ).map_err(|e| format!("Failed to create commit: {}", e))?;
//     println!("Created commit with id: {}", commit_oid);
//     Ok(())
// }


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


