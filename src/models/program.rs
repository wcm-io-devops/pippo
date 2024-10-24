use serde::{Deserialize, Serialize};

// Models for representing Cloud Manager programs
// -------------------------------------------------------------------------------------------------

/// Struct that holds the response when requesting /api/programs
#[derive(Deserialize, Serialize)]
pub struct ProgramsResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub programs_list: ProgramsList,
}

/// Model for a list of programs
#[derive(Debug, Deserialize, Serialize)]
pub struct ProgramsList {
    programs: Vec<Program>,
}

/// Model for a program and its relevant metadata
#[derive(Debug, Deserialize, Serialize)]
pub struct Program {
    id: String,
    name: String,
    #[serde(rename(deserialize = "tenantId", serialize = "tenantId"))]
    tenant_id: String,
    enabled: bool,
    status: String,
}

#[cfg(test)]
mod tests {
    use crate::models::tests::read_json_from_file;

    use super::*;

    #[test]
    fn deserialize_bearer_response() {
        let vobj: ProgramsResponse =
            read_json_from_file("test/test_programs_response.json").unwrap();

        assert_eq!(vobj.programs_list.programs.first().unwrap().id, "22222");
    }
}
