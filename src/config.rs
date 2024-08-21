use serde::{Deserialize, Serialize};
use std::fs;
use strum_macros::{EnumString, IntoStaticStr};

/// Model for a Cloud Manager connection configuration
#[derive(Debug, Deserialize)]
pub struct CloudManagerConfig {
    #[serde(skip_deserializing)]
    pub access_token: String,
    pub client_id: String,
    pub client_secret: String,
    #[serde(skip_deserializing)]
    pub jwt: String,
    pub organization_id: String,
    pub private_key: String,
    pub technical_account_id: String,
    #[serde(default = "default_scope")]
    pub scope: Scope,
    #[serde(default = "default_auth")]
    pub auth_strategy: AuthStrategy,
}

/// Possible types that the AuthStrategy can have
#[derive(Debug, Clone, Deserialize, Serialize, IntoStaticStr, EnumString, PartialEq)]
pub enum AuthStrategy {
    #[serde(rename(deserialize = "oauth2", serialize = "oauth2"))]
    OAuth2,
    #[serde(rename(deserialize = "jwt", serialize = "jwt"))]
    JWT,
}

/// Possible types that the scope can have
#[derive(Debug, Clone, Deserialize, Serialize, IntoStaticStr, EnumString, PartialEq)]
pub enum Scope {
    #[serde(rename(deserialize = "ent_cloudmgr_sdk", serialize = "ent_cloudmgr_sdk"))]
    EntCloudmgrSdk,
    #[serde(rename(deserialize = "ent_aem_cloud_api", serialize = "ent_aem_cloud_api"))]
    EntAemCloudApi,
}

/// default scope to use
fn default_scope() -> Scope {
    Scope::EntCloudmgrSdk
}
/// default Strategy to use
fn default_auth() -> AuthStrategy {
    AuthStrategy::OAuth2
}

impl CloudManagerConfig {
    /// Reads a Cloud Manager configuration from a JSON file
    ///
    /// # Arguments
    ///
    /// * `path` - String slice that holds the path to the JSON config file
    pub fn from_file(path: &str) -> Self {
        let data = fs::read_to_string(path)
            .unwrap_or_else(|_| {
                eprintln!("[ERROR] Unable to find config at path '{}'. The documentation is available at https://github.com/wcm-io-devops/pippo", path);
                std::process::exit(1)
            });
        let config: Self = serde_json::from_str(data.as_str()).expect("Invalid JSON format");
        config
    }
}
