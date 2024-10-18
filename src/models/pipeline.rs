// Models for representing Cloud Manager pipelines and descendant objects
// -------------------------------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

/// Struct that holds the response when requesting /api/programs
#[derive(Deserialize, Serialize)]
pub struct PipelinesResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub pipelines_list: PipelinesList,
}

/// Model for a list of pipelines
#[derive(Debug, Deserialize, Serialize)]
pub struct PipelinesList {
    pipelines: Vec<Pipeline>,
}

/// Model for a pipeline and its relevant metadata
#[derive(Debug, Deserialize, Serialize)]
pub struct Pipeline {
    pub name: String,
    pub status: String,
    id: String,
    #[serde(rename(deserialize = "programId", serialize = "programId"))]
    program_id: String,
}

// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tests::read_json_from_file;

    #[test]
    fn serialize_bearer_response() {
        let vobj: PipelinesResponse =
            read_json_from_file("test/test_pipeline_response.json").unwrap();

        assert_eq!(vobj.pipelines_list.pipelines.len(), 5);
    }
}
