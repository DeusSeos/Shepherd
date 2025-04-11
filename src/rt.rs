use serde::{Deserialize, Serialize};

use rancher_client::apis::{configuration::Configuration, Error, ResponseContent};
use reqwest::StatusCode;

use rancher_client::{
    apis::management_cattle_io_v3_api::{
        list_management_cattle_io_v3_role_template, ListManagementCattleIoV3RoleTemplateError,
    },
    models::{
        IoCattleManagementv3RoleTemplate, IoCattleManagementv3RoleTemplateList,
         IoK8sApimachineryPkgApisMetaV1ObjectMeta,
    },
    models::io_cattle_managementv3_role_template::Context,
};

/// Get all role templates from an endpoint using the provided configuration
///
/// # Arguments
///
/// * `configuration` - The configuration to use for the request
///
/// # Returns
///
/// * `IoCattleManagementv3RoleTemplateList` - The list of role templates
///
/// # Errors
///
/// * `Error<ListManagementCattleIoV3RoleTemplateError>` - The error that occurred while trying to get the role templates
///
pub async fn get_role_templates(
    configuration: &Configuration,
) -> Result<IoCattleManagementv3RoleTemplateList, Error<ListManagementCattleIoV3RoleTemplateError>> {
    let result = list_management_cattle_io_v3_role_template(
        configuration,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await;

    match result {
        Err(e) => Err(e),
        Ok(response_content) => {
            // Match on the status code and deserialize accordingly
            match response_content.status {
                StatusCode::OK => {
                    // Try to deserialize the content into IoCattleManagementv3RoleTemplateList (Status200 case)
                    match serde_json::from_str(&response_content.content) {
                        Ok(data) => Ok(data),
                        Err(deserialize_err) => Err(Error::Serde(deserialize_err)),
                    }
                }
                _ => {
                    // If not status 200, treat as UnknownValue
                    match serde_json::from_str::<serde_json::Value>(&response_content.content) {
                        Ok(unknown_data) => {
                            // Handle the unknown response
                            Err(Error::ResponseError(ResponseContent {
                                status: response_content.status,
                                content: response_content.content,
                                entity: Some(ListManagementCattleIoV3RoleTemplateError::UnknownValue(
                                    unknown_data,
                                )),
                            }))
                        }
                        Err(deserialize_err) => Err(Error::Serde(deserialize_err)),
                    }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RoleTemplate {
    id: String,
    name: String,
    description: String,
    context: String,
    builtin: bool,
    external: bool,
}

impl RoleTemplate {
    pub fn new(id: String, name: String, description: String, context: String, builtin: bool, external: bool) -> Self {
        RoleTemplate {
            id,
            name,
            description,
            context,
            builtin,
            external,
        }
    }
}

impl TryFrom<IoCattleManagementv3RoleTemplate> for RoleTemplate {
    type Error = &'static str;

    fn try_from(value: IoCattleManagementv3RoleTemplate) -> Result<Self, Self::Error> {
        let metadata: IoK8sApimachineryPkgApisMetaV1ObjectMeta =
            *value.metadata.ok_or("missing metadata")?;
        
        let context_str = match value.context {
            Some(Context::Project) => "project",
            Some(Context::Cluster) => "cluster",
            Some(Context::Empty) => "",
            None => return Err("missing context"),
        }.to_string();

        Ok(RoleTemplate {
            id: metadata.name.ok_or("missing name")?,
            name: value.display_name.unwrap_or_else(|| metadata.name.ok_or("missing name")?),
            description: value.description.unwrap_or_else(|| "".to_string()),
            context: context_str,
            builtin: value.builtin.unwrap_or(false),
            external: value.external.unwrap_or(false),
        })
    }
}