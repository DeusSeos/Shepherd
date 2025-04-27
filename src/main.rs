use rancher_cac::rancher_config_init;
use rancher_cac::download_current_configuration;
use rancher_cac::FileFormat;

use reqwest_middleware::ClientBuilder;
use git2::Repository;


#[tokio::main]
async fn main() {
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
    repo.remote("origin", "https://github.com/DeusSeos/rancher_config.git").unwrap();


    // push the changes to the remote repository using remotecallback and local credentials
    let mut remote_callbacks = git2::RemoteCallbacks::new();
    remote_callbacks.credentials(|_url, _username_from_url, _allowed_types| {
        // Use a deploy key for authentication
        // This assumes you have a private key at /Users/dc/.ssh/rancher_config
        let private_key = std::path::PathBuf::from("/Users/dc/.ssh/rancher_config");
        let public_key = std::path::PathBuf::from("/Users/dc/.ssh/rancher_config.pub");
        let passphrase = None;
        git2::Cred::ssh_key("git", None, &private_key, passphrase)
    }).transfer_progress(|progress| {
        println!(
            "Transferred {} bytes out of {} bytes",
            progress.received_bytes(),
            progress.total_objects()
        );
        true
    }).update_tips(|_refname, _old_oid, new_oid| {
        println!("Updated tip to {}", new_oid);
        true
    }).transfer_progress(|progress| {
        println!(
            "Transferred {} bytes out of {} bytes",
            progress.received_bytes(),
            progress.total_objects()
        );
        true
    });

    let mut proxy_options = git2::ProxyOptions::new();
    proxy_options.auto();

    let mut push_options = git2::PushOptions::new();
    push_options.remote_callbacks(remote_callbacks);
    push_options.proxy_options(proxy_options);
    let mut remote = repo.find_remote("origin").unwrap();


    // push the changes to the remote repository
    remote.push(&["refs/heads/master:refs/heads/master"], Some(&mut push_options)).unwrap();
    //

}
