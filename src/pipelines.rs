use crate::client::{AdobeConnector, CloudManagerClient};
use crate::errors::throw_adobe_api_error;
use crate::models::{Execution, Pipeline, PipelinesList, PipelinesResponse};
use crate::HOST_NAME;
use reqwest::{Error, Method, StatusCode};
use std::process;
use std::thread::sleep;
use std::time::Duration;

/// Returns a pipeline by its ID.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
/// * `pipeline_id` - A u32 that holds the pipeline ID
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/program/{programId}/pipeline/{pipelineId}
/// ```
pub async fn get_pipeline(
    client: &mut CloudManagerClient,
    program_id: u32,
    pipeline_id: u32,
) -> Result<Pipeline, Error> {
    let request_path = format!(
        "{}/api/program/{}/pipeline/{}",
        HOST_NAME, program_id, pipeline_id
    );
    let response = client
        .perform_request(Method::GET, request_path, None::<()>, None)
        .await?
        .text()
        .await?;
    let pipeline: Pipeline = serde_json::from_str(response.as_str()).unwrap_or_else(|_| {
        throw_adobe_api_error(response);
        process::exit(1);
    });
    Ok(pipeline)
}

/// Starts a new pipeline run by its pipeline_id
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
/// * `pipeline_id` - A u32 that holds the pipeline ID
///
/// # Performed API Request
///
/// ```
/// PUT https://cloudmanager.adobe.io/api/program/{programId}/pipeline/{pipelineId}/execution
/// ```
pub async fn run_pipeline(
    client: &mut CloudManagerClient,
    program_id: u32,
    pipeline_id: u32,
    ci_mode: bool,
) -> Result<Execution, Error> {
    // Check if the targeted environment is ready
    let execution: Execution;
    '_retry: loop {
        let pipeline = get_pipeline(client, program_id, pipeline_id).await.unwrap();

        if pipeline.status == "BUSY" && ci_mode {
            eprintln!(
                "{:>8} Skipped! This pipeline is currently busy and and ci mode (--ci) is active.",
                "⚠️",
            );
            process::exit(1);
        } else if pipeline.status == "BUSY" {
            eprintln!(
                "{:>8} This pipeline is currently busy. Retrying in 1 minute...",
                "⏲",
            );
            sleep(Duration::from_secs(60));
        } else {
            let request_path = format!(
                "{}/api/program/{}/pipeline/{}/execution",
                HOST_NAME, program_id, pipeline_id
            );
            let response = client
                .perform_request(Method::PUT, request_path, None::<()>, None)
                .await?
                .text()
                .await?;

            execution = serde_json::from_str(response.as_str()).unwrap_or_else(|_| {
                throw_adobe_api_error(response);
                process::exit(1);
            });
            break '_retry;
        }
    }
    Ok(execution)
}

/// Starts a new pipeline run by its pipeline_id
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
/// * `pipeline_id` - A u32 that holds the pipeline ID
///
/// # Performed API Request
///
/// ```
/// DELETE https://cloudmanager.adobe.io/api/program/{programId}/pipeline/{pipelineId}/cache
/// ```
pub async fn invalidate_pipeline_cache(
    client: &mut CloudManagerClient,
    program_id: u32,
    pipeline_id: u32,
    ci_mode: bool,
) {
    // Check if the targeted environment is ready
    '_retry: loop {
        let pipeline = get_pipeline(client, program_id, pipeline_id).await.unwrap();

        if pipeline.status == "BUSY" && ci_mode {
            eprintln!(
                "{:>8} Skipped! This pipeline is currently busy and and ci mode (--ci) is active.",
                "⚠️",
            );
            process::exit(1);
        } else if pipeline.status == "BUSY" {
            eprintln!(
                "{:>8} This pipeline is currently busy. Retrying in 1 minute...",
                "⏲",
            );
            sleep(Duration::from_secs(60));
        } else {
            let request_path = format!(
                "{}/api/program/{}/pipeline/{}/cache",
                HOST_NAME, program_id, pipeline_id
            );
            let response = client
                .perform_request(Method::DELETE, request_path, None::<()>, None)
                .await
                .unwrap();

            let status_code = response.status();
            let response_text = response.text().await;
            if status_code == StatusCode::NO_CONTENT {
                println!("{:>8} Cache of {:?} invalidated", "✍", pipeline_id);
            } else {
                throw_adobe_api_error(response_text.unwrap().clone());
                process::exit(1);
            }
            break '_retry;
        }
    }
}

/// Retrieves all pipelines.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/program/{programId}/pipelines
/// ```
pub async fn get_pipelines(
    client: &mut CloudManagerClient,
    program_id: u32,
) -> Result<PipelinesList, Error> {
    let request_path = format!("{}/api/program/{}/pipelines", HOST_NAME, program_id);
    let response = client
        .perform_request(Method::GET, request_path, None::<()>, None)
        .await?
        .text()
        .await?;
    let pipelines: PipelinesResponse =
        serde_json::from_str(response.as_str()).unwrap_or_else(|_| {
            throw_adobe_api_error(response);
            process::exit(1);
        });

    Ok(pipelines.pipelines_list)
}
