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
