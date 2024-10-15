use serde::{Deserialize, Serialize};
use std::fmt;
use strum_macros::{EnumString, IntoStaticStr};

use super::environment::EnvironmentsList;

/// Model for common cloud manager variables

/// Possible types that a variable can have
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VariableType {
    String,
    SecretString,
}

/// Model for all information about a Cloud Manager environment variable
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnvironmentVariable {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub variable_type: VariableType,
    #[serde(
        default = "EnvironmentVariableServiceType::default",
        skip_serializing_if = "environment_variable_skip_serializing"
    )]
    pub service: EnvironmentVariableServiceType,
}

/// Possible service types that an environment variable can have
#[derive(Clone, Debug, Deserialize, Serialize, IntoStaticStr, EnumString, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentVariableServiceType {
    All,
    Author,
    Publish,
    Preview,
    #[serde(other)]
    Invalid,
}

impl fmt::Display for EnvironmentVariableServiceType {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{}",
            format!("{}", serde_json::to_string(self).unwrap().to_string())
        )
    }
}
fn environment_variable_skip_serializing(t: &EnvironmentVariableServiceType) -> bool {
    *t == EnvironmentVariableServiceType::All
}

impl EnvironmentVariableServiceType {
    fn default() -> Self {
        EnvironmentVariableServiceType::All
    }
}

/// Model for all information about a Cloud Manager pipeline variable
/// Model for all information about a Cloud Manager environment variable
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PipelineVariable {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub variable_type: VariableType,
    #[serde(default = "PipelineVariableServiceType::default")]
    pub service: PipelineVariableServiceType,
}

/// Possible service types that an pipeline variable can have
#[derive(Clone, Debug, Deserialize, Serialize, IntoStaticStr, EnumString, PartialEq, Eq)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum PipelineVariableServiceType {
    Build,
    UiTest,
    FunctionalTest,
    #[serde(other)]
    Invalid,
}

impl fmt::Display for PipelineVariableServiceType {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{}",
            format!("{}", serde_json::to_string(self).unwrap().to_string())
        )
    }
}

impl PipelineVariableServiceType {
    fn default() -> Self {
        PipelineVariableServiceType::Build
    }
}

/// Struct that holds the response when requesting /api/program/{id}/environments
#[derive(Deserialize, Serialize)]
pub struct EnvironmentsResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub environments_list: EnvironmentsList,
}

/// Struct to serialize the response of requesting /api/program/{id}/environment/{id}/variables
#[derive(Debug, Deserialize, Serialize)]
pub struct EnvironmentVariablesResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub variables_list: EnvironmentVariablesList,
}

/// Struct to serialize the response of requesting /api/program/{id}/environment/{id}/variables
#[derive(Debug, Deserialize, Serialize)]
pub struct PipelineVariablesResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub variables_list: PipelineVariablesList,
}

/// Struct that holds a list of variables
#[derive(Debug, Deserialize, Serialize)]
pub struct EnvironmentVariablesList {
    pub variables: Vec<EnvironmentVariable>,
}

/// Struct that holds a list of variables
#[derive(Debug, Deserialize, Serialize)]
pub struct PipelineVariablesList {
    pub variables: Vec<PipelineVariable>,
}
