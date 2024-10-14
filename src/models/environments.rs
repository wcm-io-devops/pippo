use serde::{Deserialize, Serialize};

// Models for representing Cloud Manager environments and descendant objects
// -------------------------------------------------------------------------------------------------

/// Struct that holds the response when requesting /api/program/{id}/environments
#[derive(Deserialize, Serialize)]
pub struct EnvironmentsResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub environments_list: EnvironmentsList,
}

/// Model for a list of environments
#[derive(Debug, Deserialize, Serialize)]
pub struct EnvironmentsList {
    environments: Vec<Environment>,
}

/// Model for an environment and its relevant metadata
#[derive(Debug, Deserialize, Serialize)]
pub struct Environment {
    pub name: String,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    env_type: String,
    pub status: String,
    id: String,
    #[serde(rename(deserialize = "programId", serialize = "programId"))]
    program_id: String,
}
