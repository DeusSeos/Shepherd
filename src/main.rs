


use rancher_cac::git::push_repo_to_remote;
use rancher_cac::git::init_git_repo_with_main_branch;
use rancher_cac::rancher_config_init;
use rancher_cac::download_current_configuration;
use rancher_cac::FileFormat;

use reqwest_middleware::ClientBuilder;


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
    download_current_configuration(configuration, &path, file_format).await;

    // set up the remote url to be git@github.com/DeusSeos/rancher_config.git
    let remote_url = "git@github.com:DeusSeos/rancher_config.git";

    init_git_repo_with_main_branch(&path, &remote_url).unwrap();

    // push the repo to the remote
    // push_repo_to_remote(&path, &remote_url).unwrap();

    Ok(())

}
