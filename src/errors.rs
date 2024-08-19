use colored::*;
use serde::{Deserialize, Serialize};

/// Struct that's used to deserialize Adobe API errors
#[derive(Debug, Deserialize, Serialize)]
pub struct AdobeApiError {
    pub status: u32,
    #[serde(rename(deserialize = "type"))]
    pub error_type: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
    #[serde(
        rename(deserialize = "invalidParams"),
        skip_serializing_if = "Option::is_none"
    )]
    pub invalid_params: Option<Vec<AdobeApiErrorInvalidParams>>,
    #[serde(
        rename(deserialize = "missingParams"),
        skip_serializing_if = "Option::is_none"
    )]
    pub missing_params: Option<Vec<AdobeApiErrorMissingParams>>,
}

/// Struct that's used to deserialize Adobe API error invalid parameters
#[derive(Debug, Deserialize, Serialize)]
pub struct AdobeApiErrorInvalidParams {
    pub name: String,
    pub reason: String,
}

/// Struct that's used to deserialize Adobe API error missing parameters
#[derive(Debug, Deserialize, Serialize)]
pub struct AdobeApiErrorMissingParams {
    pub name: String,
    #[serde(rename(deserialize = "type"))]
    pub parameter_type: String,
}

/// Throws an AdobeApiError.
///
/// # Arguments
///
/// * `error_response` - String that contains the returned error message from Adobe's API
pub fn throw_adobe_api_error(error_response: String) {
    let api_error = serde_json::from_str::<AdobeApiError>(error_response.as_str()).unwrap();
    eprintln!(
        "{}\n{}",
        "❌ API Error; check output below.".red().bold(),
        serde_json::to_string_pretty(&api_error).unwrap().magenta()
    );
}
