use serde::{Deserialize, Serialize};

use rancher_client::apis::{configuration::Configuration, Error, ResponseContent};
use reqwest::StatusCode;

use rancher_client::{
    apis::management_cattle_io_v3_api::{
        list_management_cattle_io_v3_role_template, ListManagementCattleIoV3RoleTemplateError,
    },
    models::{
        IoCattleManagementv3RoleTemplate, IoCattleManagementv3RoleTemplateList,
        IoCattleManagementv3GlobalRoleRulesInner,
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
    builtin: bool,
    cluster_creator_default: bool,
    context: String,
    description: String,
    display_name: String,
    external: bool,
    hidden: bool,
    locked: bool,
    id: String,
    project_creator_default: bool,
    role_template_names: Vec<String>,
    rules: Vec<IoCattleManagementv3GlobalRoleRulesInner>,
}

impl RoleTemplate {
    pub fn new(
        builtin: bool,
        cluster_creator_default: bool,
        context: String,
        description: String,
        display_name: String,
        external: bool,
        hidden: bool,
        locked: bool,
        id: String,
        project_creator_default: bool,
        role_template_names: Vec<String>,
        rules: Vec<IoCattleManagementv3GlobalRoleRulesInner>,
    ) -> RoleTemplate {
        RoleTemplate {
            builtin,
            cluster_creator_default,
            context,
            description,
            display_name,
            external,
            hidden,
            locked,
            id,
            project_creator_default,
            role_template_names,
            rules,
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

        let rules: Vec<IoCattleManagementv3GlobalRoleRulesInner> =
            value.rules.ok_or("missing rules")?;

        let role_template_names: Vec<String> =
            value
                .role_template_names
                .ok_or("missing role_template_names")?;
        let builtin: bool = value.builtin.unwrap_or(false);
        let cluster_creator_default: bool = value.cluster_creator_default.unwrap_or(false);
        let description: String = value.description.unwrap_or_default();
        let display_name: String = value.display_name.unwrap_or_default();
        let external: bool = value.external.unwrap_or(false);
        let hidden: bool = value.hidden.unwrap_or(false);
        let locked: bool = value.locked.unwrap_or(false);
        let project_creator_default: bool = value.project_creator_default.unwrap_or(false);

        Ok(RoleTemplate {
            builtin,
            cluster_creator_default,
            context: context_str,
            description,
            display_name,
            external,
            hidden,
            locked,
            id: metadata.name.ok_or("missing metadata.name")?,
            project_creator_default,
            role_template_names,
            rules,
        })
    }
}

