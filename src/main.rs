
use std::ops::Deref;
use std::path::Path;

use rancher_cac::rancher_config_init;
use rancher_cac::download_current_configuration;
use rancher_cac::FileFormat;

use reqwest_middleware::ClientBuilder;

use git2::{Cred, PushOptions, RemoteCallbacks, ProxyOptions, Repository};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration using
    // the provided URL and token
    // URL: https://rancher.rd.localhost/v3
    // Token: token-xxzn4:mlcl7q4m2vl6mq8hfzdffh5f5fh4wfzqqhzbm52bqzkpmhdg2c7bf7

    let mut configuration = rancher_config_init("https://rancher.rd.localhost/", "token-xxzn4:mlcl7q4m2vl6mq8hfzdffh5f5fh4wfzqqhzbm52bqzkpmhdg2c7bf7");

    // modify the configuration client to allow self-signed certificates
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    configuration.client = ClientBuilder::new(client).build();
        
    // Create a path to the folder where the configuration will be saved
    let path = std::path::PathBuf::from("/tmp/rancher_config");
                        
    // Create a file format to save the configuration in
    let file_format = FileFormat::Yaml;

    // Download the current configuration from the Rancher API
    download_current_configuration(configuration, path, file_format).await;

    // initialize a git repository in the folder
    let repo = Repository::init("/tmp/rancher_config").unwrap();

    // commit the changes
    let mut index = repo.index().unwrap();
    index.add_all(["*"], git2::IndexAddOption::DEFAULT, None).unwrap();
    let oid = index.write().unwrap();
    let signature = repo.signature().unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let commit_oid = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    ).unwrap();
    println!("Created commit with id: {}", commit_oid);

    // set up the remote repository to be https://github.com/DeusSeos/rancher_config.git and then push the changes
    repo.remote("origin", "git@github.com:DeusSeos/rancher_config.git").unwrap();

    // Setup RemoteCallbacks
    let mut remote_callbacks = RemoteCallbacks::new();
    remote_callbacks
        .credentials(|_url, username_from_url, _allowed_types| {
            let username = username_from_url.unwrap_or("git");
            let private_key = Path::new("/Users/dc/.ssh/rancher_config");
            Cred::ssh_key(username, None, private_key, None)
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

    // Find remote
    let mut remote = repo.find_remote("origin")
        .map_err(|e| format!("Failed to find remote 'origin': {}", e))?;

    // Perform push
    remote.push(
        &["refs/heads/master:refs/heads/master"],
        Some(&mut push_options),
    ).map_err(|e| format!("Failed to push to remote: {}", e))?;

    println!("Push completed successfully.");
    //

    Ok(())

}
