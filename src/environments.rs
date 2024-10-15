use crate::client::{AdobeConnector, CloudManagerClient};
use crate::errors::throw_adobe_api_error;
use crate::models::environment::{Environment, EnvironmentsList};
use crate::models::variables::EnvironmentsResponse;
use crate::HOST_NAME;
use reqwest::{Error, Method};
use std::process;

/// Retrieves all environments of a given program ID.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/program/{program_id}/environments
/// ```
pub async fn get_environments(
    client: &mut CloudManagerClient,
    program_id: u32,
) -> Result<EnvironmentsList, Error> {
    let request_path = format!("{}/api/program/{}/environments", HOST_NAME, program_id);
    let response = client
        .perform_request(Method::GET, request_path, None::<()>, None)
        .await?
        .text()
        .await?;
    let environments: EnvironmentsResponse = serde_json::from_str(response.as_str())
        .unwrap_or_else(|_| {
            throw_adobe_api_error(response);
            process::exit(1);
        });
    Ok(environments.environments_list)
}

/// Retrieves a single environment.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
/// * `env_id` - A u32 that holds the environment ID
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/program/{program_id}/environment/{env_id}
/// ```
pub async fn get_environment(
    client: &mut CloudManagerClient,
    program_id: u32,
    env_id: u32,
) -> Result<Environment, Error> {
    let request_path = format!(
        "{}/api/program/{}/environment/{}",
        HOST_NAME, program_id, env_id
    );
    let response = client
        .perform_request(Method::GET, request_path, None::<()>, None)
        .await?
        .text()
        .await?;
    let environment: Environment = serde_json::from_str(response.as_str()).unwrap_or_else(|_| {
        throw_adobe_api_error(response);
        process::exit(1);
    });
    Ok(environment)
}
