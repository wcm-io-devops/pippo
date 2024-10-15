use serde::{Deserialize, Serialize};
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
