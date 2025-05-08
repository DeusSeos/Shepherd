#[allow(unused_imports)]


use rancher_cac::git::push_repo_to_remote;
use rancher_cac::git::{init_git_repo_with_main_branch, commit_changes};
use rancher_cac::rancher_config_init;
use rancher_cac::download_current_configuration;
use rancher_cac::FileFormat;
use rancher_cac::load_project;

use rancher_cac::project::find_project;


use chrono;

use reqwest_middleware::ClientBuilder;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration using
    // the provided URL and token
    // URL: https://rancher.rd.localhost/v3
    // Token: token-xxzn4:mlcl7q4m2vl6mq8hfzdffh5f5fh4wfzqqhzbm52bqzkpmhdg2c7bf7

    let mut configuration = rancher_config_init("https://rancher.rd.localhost", "token-xxzn4:mlcl7q4m2vl6mq8hfzdffh5f5fh4wfzqqhzbm52bqzkpmhdg2c7bf7");

    // modify the configuration client to allow self-signed certificates
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    configuration.client = ClientBuilder::new(client).build();
        
    // Create a path to the folder where the configuration will be saved
    // let path = std::path::PathBuf::from("/tmp/rancher_config");

    let path = std::path::PathBuf::from("/Users/dc/Documents/Rust/rancher_config");
                        
    // Create a file format to save the configuration in
    let file_format = FileFormat::Yaml;

    // Download the current configuration from the Rancher API
    // download_current_configuration(&configuration, &path, file_format).await;

    // set up the remote url to be git@github.com/DeusSeos/rancher_config.git
    // let remote_url = "git@github.com:DeusSeos/rancher_config.git";


    // // Initialize a git repository in the path or if error, commit a change with current datetime
    // init_git_repo_with_main_branch(&path, &remote_url).unwrap_or_else(|_| {
    //     // commit a change with current datetime
    //     let now = chrono::Utc::now();
    //     let datetime = now.format("%Y-%m-%d %H:%M:%S").to_string();
    //     let message = format!("Updated configuration at {}", datetime);
    //     commit_changes(&path, &message).unwrap();
    //     println!("Error initializing git repository, committed changes with message: {}", message);
        
    // });

    let cluster_id = "local";
    let project_id = "p-w82pc";

    // load project from path
    let project = load_project(&path, configuration.base_path.clone().as_str(),  cluster_id, project_id, FileFormat::Yaml);

    // print the project
    println!("{:#?}", project);

    // fetch the project from cluster
    let rancher_project = find_project(&configuration, cluster_id, project_id).await.unwrap();

    // print the project
    println!("{:#?}", rancher_project);

    // check equality
    if project == rancher_project {
        println!("Project is equal");
    } else {
        println!("Project is not equal");
    }

    // push the repo to the remote
    // push_repo_to_remote(&path, &remote_url).unwrap();

    

    Ok(())

}
