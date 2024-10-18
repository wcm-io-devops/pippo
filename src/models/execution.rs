use serde::{Deserialize, Serialize};

/// Model for a list of programs
#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutionList {
    #[serde(rename = "executions")]
    list: Vec<Execution>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResponse {
    #[serde(rename = "_embedded")]
    pub execution_list: ExecutionList,
    #[serde(rename = "_totalNumberOfItems")]
    pub total_number_of_items: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    pub id: String,
    pub program_id: String,
    pub pipeline_id: String,
    trigger: String,
    user: String,
    pub status: String,
    created_at: Option<String>,
    updated_at: Option<String>,
    pipeline_type: String,
    pipeline_execution_mode: String,
    finished_at: Option<String>,
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::tests::read_json_from_file;

    #[test]
    fn serialize_domain_config() {
        let vobj: ExecutionResponse =
            read_json_from_file("test/test_execution_response.json").unwrap();

        assert_eq!(vobj.execution_list.list.first().unwrap().id, "66666");
    }
}
