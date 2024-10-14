use crate::client::{AdobeConnector, CloudManagerClient};
use crate::errors::throw_adobe_api_error;
use crate::models::program::{ProgramsList, ProgramsResponse};
use crate::HOST_NAME;
use reqwest::{Error, Method};
use std::process;

/// Retrieves all programs.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/programs
/// ```
pub async fn get_programs(client: &mut CloudManagerClient) -> Result<ProgramsList, Error> {
    let request_path = format!("{}/api/programs", HOST_NAME);
    let response = client
        .perform_request(Method::GET, request_path, None::<()>, None)
        .await?
        .text()
        .await?;
    let programs: ProgramsResponse = serde_json::from_str(response.as_str()).unwrap_or_else(|_| {
        throw_adobe_api_error(response);
        process::exit(1);
    });

    Ok(programs.programs_list)
}
