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

