use serde::{Deserialize, Serialize};

/// Model for all information about a Cloud Manager environment variable
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Variable {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub variable_type: VariableType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Possible types that a variable can have
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VariableType {
    String,
    SecretString,
}

/// Struct to serialize the response of requesting /api/program/{id}/environment/{id}/variables
#[derive(Debug, Deserialize, Serialize)]
pub struct VariablesResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub variables_list: VariablesList,
}

/// Struct that holds a list of variables
#[derive(Debug, Deserialize, Serialize)]
pub struct VariablesList {
    pub variables: Vec<Variable>,
}
