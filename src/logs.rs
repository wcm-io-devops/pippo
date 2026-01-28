use std::io::Cursor;
use std::process;
use std::thread::sleep;
use std::time::Duration;

use chrono::NaiveDate;
use colored::*;
use log::debug;
use reqwest::{Error, Method, StatusCode};

use crate::client::{AdobeConnector, CloudManagerClient};
use crate::errors::throw_adobe_api_error;
use crate::models::log::{LogTailResponse, LogType, ServiceType};
use crate::HOST_NAME;

/// Downloads the specified log.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
/// * `env_id` - A u32 that holds the environment ID
/// * `service` - Name of the service type - can be either author, publish, dispatcher, or preview_dispatcher
/// * `logname` - Name of the logfile - can be either aemaccess, aemdispatcher, aemerror, aemrequest, cdn, httpdaccess, or httpderror
/// * `date` - Date you want to retrieve the logs from, in the format YYYY-MM-DD
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/program/{program_id}/environment/{env_id}/logs/download
/// ```
pub async fn download_log(
    client: &mut CloudManagerClient,
    program_id: u32,
    env_id: u32,
    service: ServiceType,
    logname: LogType,
    date: NaiveDate,
) -> Result<String, Error> {
    // Convert date to String, since query parameters must be all of the same type
    let naive_date = date.to_string();

    let query_parameters = vec![
        ("service", service.clone().into()),
        ("name", logname.clone().into()),
        ("date", naive_date.as_str()),
    ];

    let request_path = format!(
        "{}/api/program/{}/environment/{}/logs/download",
        HOST_NAME, program_id, env_id
    );

    let response = client
        .perform_request(
            Method::GET,
            request_path,
            None::<()>,
            Some(query_parameters),
        )
        .await?;

    match response.status() {
        StatusCode::NOT_FOUND => {
            eprintln!(
                "{}",
                "❌ The requested logfile was not found. Check your parameters.".red()
            );
            process::exit(1);
        }
        StatusCode::OK => {
            let download = response.bytes().await?;
            // Save archive to file in working directory
            let filename = format!(
                "{}_{}-{}_{}.log.gz",
                date,
                env_id,
                Into::<&str>::into(&service),
                Into::<&str>::into(&logname),
            );
            let mut file = std::fs::File::create(&filename).unwrap();
            let mut content = Cursor::new(download);
            std::io::copy(&mut content, &mut file).unwrap();

            Ok(filename)
        }
        _ => {
            eprintln!("wtf? -> {}", response.status());
            unreachable!();
        }
    }
}

/// Tails the specified log.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
/// * `env_id` - A u32 that holds the environment ID
/// * `service` - Name of the service type - can be either author, publish, dispatcher, or preview_dispatcher
/// * `logname` - Name of the logfile - can be either aemaccess, aemdispatcher, aemerror, aemrequest, cdn, httpdaccess, or httpderror
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/program/{program_id}/environment/{env_id}/logs/download
/// ```
pub async fn tail_log(
    client: &mut CloudManagerClient,
    program_id: u32,
    env_id: u32,
    service: ServiceType,
    logname: LogType,
) -> Result<(), Error> {
    println!("{}", "Tailing requested log (exit with Ctrl-C)".yellow());
    println!(
        "{}", "⚠ Be aware that Adobe doesn't provide logs in realtime, so it might take a couple of seconds before logs start showing up.".yellow()
    );

    // -> get log path from API

    let tail_url = get_tail_log_url(client, program_id, env_id, service, logname)
        .await
        .unwrap();

    let reqwest_client = reqwest::Client::new();
    let init_response = reqwest_client.head(&tail_url).send().await?;

    let mut last_content_length: i64 = 0;

    match init_response.status() {
        StatusCode::NOT_FOUND => {
            eprintln!(
                "{}",
                "❌ The requested logfile was not found. Check your parameters.".red()
            );
            process::exit(1);
        }
        StatusCode::OK => {
            debug!("Init response: {:?}", init_response);
            let content_length = init_response
                .headers()
                .get("content-length")
                .unwrap()
                .to_str();
            last_content_length = content_length.unwrap().to_owned().parse::<i64>().unwrap();
            debug!("initial Content Length: {:?}", last_content_length);
        }
        _ => {
            eprintln!("{}: {}", "❌ API Error".red(), init_response.status());
        }
    }

    // Now we can start printing what's being added to the logfile.
    loop {
        let range_header_value = format!("bytes={}-", last_content_length);

        debug!("range_header_value: {:?}", range_header_value);
        let response = reqwest_client
            .get(&tail_url)
            .header("Range", range_header_value)
            .send()
            .await?;
        let current_content_length: i64 = response.content_length().unwrap() as i64;

        debug!("Content Length: {:?}", current_content_length);
        debug!("response.status(): {:?}", response.status());

        match response.status() {
            StatusCode::PARTIAL_CONTENT => {
                let buffer: String = response.text().await?;
                let current_log_lines = buffer.split('\n').collect::<Vec<_>>();

                for line in current_log_lines {
                    // Don't print the trailing \n of the logfile
                    if line.is_empty() {
                        continue;
                    }
                    println!("{}", line);
                }
                // sum with current content length because we need a new range start value
                // for our next request
                last_content_length += current_content_length;
                sleep(Duration::from_secs(5));
            }
            StatusCode::RANGE_NOT_SATISFIABLE => {
                // no new content
                sleep(Duration::from_secs(5));
            }
            _ => {
                eprintln!("{}: {}", "❌ API Error".red(), response.status());
            }
        }
    }
}

///  Gets the Url of the log we want to tail
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
/// * `env_id` - A u32 that holds the environment ID
/// * `service` - Name of the service type - can be either author, publish, dispatcher, or preview_dispatcher
/// * `logname` - Name of the logfile - can be either aemaccess, aemdispatcher, aemerror, aemrequest, cdn, httpdaccess, or httpderror
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/program/{program_id}/environment/{env_id}/logs/
/// ```
pub async fn get_tail_log_url(
    client: &mut CloudManagerClient,
    program_id: u32,
    env_id: u32,
    service: ServiceType,
    logname: LogType,
) -> Result<String, Error> {
    let query_parameters = vec![
        ("service", service.clone().into()),
        ("name", logname.clone().into()),
        ("days", "2"),
    ];

    let request_path = format!(
        "{}/api/program/{}/environment/{}/logs",
        HOST_NAME, program_id, env_id
    );

    let response_obj = client
        .perform_request(
            Method::GET,
            request_path.clone(),
            None::<()>,
            Some(query_parameters.clone()),
        )
        .await?
        .text()
        .await?;
    let response: LogTailResponse =
        serde_json::from_str(response_obj.as_str()).unwrap_or_else(|_| {
            throw_adobe_api_error(response_obj);
            process::exit(1);
        });
    match &response.embedded.downloads[0]
        .links
        .http_ns_adobe_com_adobecloud_rel_logs_tail
    {
        Some(value) => {
            // returning the log tail url
            Ok(value.href.to_owned())
        }
        None => {
            unreachable!();
        }
    }
}
