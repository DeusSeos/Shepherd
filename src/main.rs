#![allow(unused_variables)]
#![allow(unused_imports)]

use rancher_cac::{download_current_configuration, load_project, rancher_config_init, remove_path_and_return, create_json_patch, FileFormat};
use rancher_cac::git::{commit_changes, init_git_repo_with_main_branch, push_repo_to_remote};
use rancher_cac::project::{clean_up_project, update_project, find_project, show_project_diff, show_text_diff, Project};

use chrono;

use rancher_client::models::IoCattleManagementv3Project;
use reqwest_middleware::ClientBuilder;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration using
    // the provided URL and token
    // URL: https://rancher.rd.localhost/v3
    // Token: token-xxzn4:mlcl7q4m2vl6mq8hfzdffh5f5fh4wfzqqhzbm52bqzkpmhdg2c7bf7

    let mut configuration = rancher_config_init(
        "https://rancher.rd.localhost",
        "token-xxzn4:mlcl7q4m2vl6mq8hfzdffh5f5fh4wfzqqhzbm52bqzkpmhdg2c7bf7",
    );

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

    let download = false;

    if download {
        // Download the current configuration from the Rancher API
        download_current_configuration(&configuration, &path, &file_format).await;

        // set up the remote url to be git@github.com/DeusSeos/rancher_config.git
        let remote_url = "git@github.com:DeusSeos/rancher_config.git";

        // Initialize a git repository in the path or if error, commit a change with current datetime
        init_git_repo_with_main_branch(&path, &remote_url).unwrap_or_else(|_| {
            // commit a change with current datetime
            let now = chrono::Utc::now();
            let datetime = now.format("%Y-%m-%d %H:%M:%S").to_string();
            let message = format!("Updated configuration at {}", datetime);
            commit_changes(&path, &message).unwrap();
            println!(
                "Error initializing git repository, committed changes with message: {}",
                message
            );
        });
    }

    let cluster_id = "local";
    let project_id = "p-w82pc";

    // load project from path
    let loaded_project = load_project(
        &path,
        configuration.base_path.clone().as_str(),
        cluster_id,
        project_id,
        file_format,
    )
    .await;

    // let loaded_project_json = json!({ "cluster_name": "local", "id": "p-w82pc", "description": "blah blah blah meh", "annotations": { "lifecycle.cattle.io/create.mgmt-project-rbac-remove": "true", "authz.management.cattle.io/creator-role-bindings": "{\"created\":[\"project-owner\"],\"required\":[\"project-owner\"]}", "lifecycle.cattle.io/create.project-namespace-auth_local": "true", "field.cattle.io/creatorId": "user-jzh8l", "fee": "fii", "fo": "fum" }, "labels": { "foo": "bar2", "changeRequest": "CHG829112", "cattle.io/creator": "norman", "another": "two" }, "container_default_resource_limit": { "limitsCpu": "11m", "limitsMemory": "32Mi", "requestsCpu": "5m", "requestsMemory": "16Mi" }, "display_name": "5", "namespace": "local", "resource_version": "1246368", "uid": "2bffd4eb-2c79-4153-9124-da1420ba57bc", "namespace_default_resource_quota": { "limit": { "limitsCpu": "500m", "limitsMemory": "512Mi", "requestsCpu": "5m", "requestsMemory": "32Mi" } }, "resource_quota": { "limitsCpu": "1000m", "limitsMemory": "1024Mi", "requestsCpu": "25m", "requestsMemory": "128Mi" } });
    // let loaded_project = serde_json::from_value::<Project>(loaded_project_json).unwrap();

    // fetch the project from cluster
    let live_rancher_project = find_project(&configuration, cluster_id, project_id)
        .await
        .unwrap();

    // convert to pretty json
    // let live_rancher_json = serde_json::to_string_pretty(&live_rancher_project).unwrap();


    // let live_rancher_json = json!({ "apiVersion": "management.cattle.io/v3", "kind": "Project", "metadata": { "annotations": { "fee": "fii", "lifecycle.cattle.io/create.mgmt-project-rbac-remove": "true", "field.cattle.io/creatorId": "user-jzh8l", "lifecycle.cattle.io/create.project-namespace-auth_local": "true", "fo": "fum", "authz.management.cattle.io/creator-role-bindings": "{\"created\":[\"project-owner\"],\"required\":[\"project-owner\"]}" }, "creationTimestamp": "2024-12-22T15:43:56Z", "finalizers": [ "clusterscoped.controller.cattle.io/project-namespace-auth_local", "controller.cattle.io/mgmt-project-rbac-remove" ], "generateName": "p-", "generation": 37, "labels": { "changeRequest": "CHG829112", "cattle.io/creator": "norman", "another": "three", "foo": "bar2" }, "managedFields": [ { "apiVersion": "management.cattle.io/v3", "fieldsType": "FieldsV1", "fieldsV1": { "f:metadata": { "f:annotations": { "f:lifecycle.cattle.io/create.project-namespace-auth_local": {} }, "f:finalizers": { ".": {}, "v:\"clusterscoped.controller.cattle.io/project-namespace-auth_local\"": {} } } }, "manager": "rancher-v2.10.1-rbac-handler-base", "operation": "Update", "time": "2024-12-22T15:43:56Z" }, { "apiVersion": "management.cattle.io/v3", "fieldsType": "FieldsV1", "fieldsV1": { "f:metadata": { "f:annotations": { ".": {}, "f:fee": {}, "f:field.cattle.io/creatorId": {}, "f:fo": {} }, "f:generateName": {}, "f:labels": { ".": {}, "f:another": {}, "f:cattle.io/creator": {}, "f:changeRequest": {}, "f:foo": {} } }, "f:spec": { ".": {}, "f:clusterName": {}, "f:containerDefaultResourceLimit": { ".": {}, "f:limitsCpu": {}, "f:limitsMemory": {}, "f:requestsCpu": {}, "f:requestsMemory": {} }, "f:description": {}, "f:displayName": {}, "f:namespaceDefaultResourceQuota": { ".": {}, "f:limit": { "f:limitsCpu": {}, "f:limitsMemory": {}, "f:requestsCpu": {}, "f:requestsMemory": {} } }, "f:resourceQuota": { ".": {}, "f:limit": { "f:limitsCpu": {}, "f:limitsMemory": {}, "f:requestsCpu": {}, "f:requestsMemory": {} } } } }, "manager": "Go-http-client", "operation": "Update", "time": "2025-05-08T21:25:36Z" }, { "apiVersion": "management.cattle.io/v3", "fieldsType": "FieldsV1", "fieldsV1": { "f:metadata": { "f:annotations": { "f:authz.management.cattle.io/creator-role-bindings": {}, "f:lifecycle.cattle.io/create.mgmt-project-rbac-remove": {} }, "f:finalizers": { "v:\"controller.cattle.io/mgmt-project-rbac-remove\"": {} } }, "f:spec": { "f:namespaceDefaultResourceQuota": { "f:limit": {} }, "f:resourceQuota": { "f:limit": {}, "f:usedLimit": {} } }, "f:status": { "f:conditions": {} } }, "manager": "rancher", "operation": "Update", "time": "2025-05-08T21:25:36Z" } ], "name": "p-w82pc", "namespace": "local", "resourceVersion": "1246447", "uid": "2bffd4eb-2c79-4153-9124-da1420ba57bc" }, "spec": { "clusterName": "local", "containerDefaultResourceLimit": { "limitsCpu": "10m", "limitsMemory": "32Mi", "requestsCpu": "5m", "requestsMemory": "16Mi" }, "description": "blah blah blah meh", "displayName": "5", "namespaceDefaultResourceQuota": { "limit": { "limitsCpu": "500m", "limitsMemory": "512Mi", "requestsCpu": "5m", "requestsMemory": "32Mi" } }, "resourceQuota": { "limit": { "limitsCpu": "1000m", "limitsMemory": "1024Mi", "requestsCpu": "25m", "requestsMemory": "128Mi" }, "usedLimit": {} } }, "status": { "conditions": [ { "lastUpdateTime": "2024-12-22T15:43:56Z", "status": "True", "type": "BackingNamespaceCreated" }, { "lastUpdateTime": "2024-12-22T15:43:57Z", "status": "True", "type": "CreatorMadeOwner" }, { "lastUpdateTime": "2024-12-22T15:43:56Z", "status": "True", "type": "InitialRolesPopulated" } ] } });
    // let live_rancher_project = serde_json::from_value::<IoCattleManagementv3Project>(live_rancher_json).unwrap();

    


    // check equality
    if loaded_project == live_rancher_project {
        // println!("Project is equal");
    } else {
        // println!("Project is not equal");
        // convert the project to IoCattleManagementv3Project
        let loaded_rancher_project: IoCattleManagementv3Project =
            loaded_project.clone().try_into().unwrap();

        // convert the live project to Value
        let mut live_project_value: serde_json::Value = serde_json::to_value(live_rancher_project).unwrap();
        let mut loaded_project_value: serde_json::Value = serde_json::to_value(loaded_rancher_project).unwrap();

        clean_up_project(&mut live_project_value);
        clean_up_project(&mut loaded_project_value);

        // println!("live up {:#?}", live_project_value);
        // println!("loaded up {:#?}", loaded_project_value);

        let patch = create_json_patch::<IoCattleManagementv3Project>(&live_project_value, &loaded_project_value);
        println!("patch {:#?}", patch);

        // update the live project with the patch
        let updated_project = update_project(&configuration, cluster_id, project_id, patch).await;

        println!("updated project {:#?}", updated_project);

        // show_text_diff(&project, &rancher_project);
    }

    // push the repo to the remote
    // push_repo_to_remote(&path, &remote_url).unwrap();

    Ok(())
}
