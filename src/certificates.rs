use crate::client::{AdobeConnector, CloudManagerClient};
use crate::errors::throw_adobe_api_error;
use crate::models::certificates::{CertificateList, CertificateResponse};
use crate::HOST_NAME;
use reqwest::{Error, Method};
use std::process;
use std::str;

/// Retrieves all certificates.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/program/{}/certificates
/// ```
pub async fn get_certificates(
    client: &mut CloudManagerClient,
    program_id: u32,
    start: &u32,
    limit: &u32,
) -> Result<CertificateList, Error> {
    let request_path = format!("{}/api/program/{}/certificates", HOST_NAME, program_id);
    let query_start: &str = &start.to_string();
    let query_limit: &str = &limit.to_string();
    let query_parameters = vec![("start", query_start), ("limit", query_limit)];
    let response = client
        .perform_request(
            Method::GET,
            request_path,
            None::<()>,
            None
        )
        .await?
        .text()
        .await?;
    let certificates: CertificateResponse = serde_json::from_str(response.as_str()).unwrap_or_else(|_| {
        throw_adobe_api_error(response);
        process::exit(1);
    });

    Ok(certificates.certificate_list)
}