use serde::{Deserialize, Serialize};
/// Model for a list of environments
#[derive(Debug, Deserialize, Serialize)]
pub struct EnvironmentsList {
    environments: Vec<Environment>,
}
/// Struct that holds the response when requesting /api/program/{id}/environments
#[derive(Deserialize, Serialize)]
pub struct EnvironmentsResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub environments_list: EnvironmentsList,
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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::tests::read_json_from_file;

    #[test]
    fn deserialize_environments_response() {
        let vobj: EnvironmentsResponse =
            read_json_from_file("test/test_environment_response.json").unwrap();

        assert_eq!(
            vobj.environments_list.environments.first().unwrap().id,
            "222222"
        );
    }
}
