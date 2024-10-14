use crate::client::{AdobeConnector, CloudManagerClient};
use crate::errors::throw_adobe_api_error;
use crate::models::executions::{ExecutionList, ExecutionResponse};
use crate::HOST_NAME;
use reqwest::{Error, Method};
use std::process;

/// Retrieves all Executions of a pipeline.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - program id
/// * `pipeline_id` - pipeline id
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/program/{}/pipeline/{}/executions
/// ```
pub async fn get_executions(
    client: &mut CloudManagerClient,
    program_id: u32,
    pipeline_id: u32,
) -> Result<ExecutionList, Error> {
    let request_path = format!(
        "{}/api/program/{}/pipeline/{}/executions",
        HOST_NAME, program_id, pipeline_id
    );
    let response = client
        .perform_request(Method::GET, request_path, None::<()>, None)
        .await?
        .text()
        .await?;

    let execution_response: ExecutionResponse = serde_json::from_str(response.as_str())
        .unwrap_or_else(|_| {
            throw_adobe_api_error(response);
            process::exit(1);
        });

    Ok(execution_response.execution_list)
}
