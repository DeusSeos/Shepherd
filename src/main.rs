use rancher_cac::rancher_config_init;
use rancher_cac::download_current_configuration;
use rancher_cac::FileFormat;


#[tokio::main]
async fn main() {
    // Create configuration using
    // the provided URL and token
    // URL: https://rancher.rd.localhost/v3
    // Token: 

    let configuration = rancher_config_init("https://rancher.rd.localhost/", "");
    // Create a path to the folder where the configuration will be saved
    let path = std::path::PathBuf::from("/tmp/rancher_config");
                        
    // Create a file format to save the configuration in
    let file_format = FileFormat::Json;

    // Download the current configuration from the Rancher API
    download_current_configuration(configuration, path, file_format).await

}
