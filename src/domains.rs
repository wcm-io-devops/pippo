use crate::client::{AdobeConnector, CloudManagerClient};
use crate::errors::throw_adobe_api_error;
use crate::models::config::YamlConfig;
use crate::models::domain::{CreateDomainResponse, DomainList, DomainResponse, MinimumDomain};
use crate::HOST_NAME;
extern crate uuid;
use colored::Colorize;
use reqwest::{Error, Method, StatusCode};
use std::process;
use std::str;
use uuid::Uuid;

/// Retrieves all domains.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/program/{}/domainNames
/// ```
pub async fn get_domains(
    client: &mut CloudManagerClient,
    program_id: u32,
    start: &u32,
    limit: &u32,
) -> Result<DomainList, Error> {
    let request_path = format!("{}/api/program/{}/domainNames", HOST_NAME, program_id);
    let query_start: &str = &start.to_string();
    let query_limit: &str = &limit.to_string();
    let query_parameters = vec![("start", query_start), ("limit", query_limit)];
    let response = client
        .perform_request(
            Method::GET,
            request_path,
            None::<()>,
            Some(query_parameters),
        )
        .await?
        .text()
        .await?;
    let domains: DomainResponse = serde_json::from_str(response.as_str()).unwrap_or_else(|_| {
        throw_adobe_api_error(response);
        process::exit(1);
    });

    Ok(domains.domain_list)
}

/// Created domains that are read from a given YAML file.
///
/// # Arguments
///
/// * `file_path` - String slice that holds the path to the YAML variables config
/// * `client` - A mutable reference to a CloudManagerClient instance
pub async fn create_domains(
    file_path: String,
    client: &mut CloudManagerClient,
) -> Result<StatusCode, Error> {
    let input = std::fs::read_to_string(file_path).expect("Unable to read file");
    let input: YamlConfig = serde_yaml::from_str(input.as_str()).unwrap_or_else(|err| {
        eprintln!("{} {}", "❌ Malformed YAML: ".red(), err);
        process::exit(1);
    });
    let mut ret_value = 0;
    let programs: Vec<crate::models::config::ProgramsConfig> = input.programs;
    for d in &programs {
        println!("☁ Program: {}", d.id,);
        if let Some(environments_vec) = &d.environments {
            for e in environments_vec {
                if let Some(domain_vec) = &e.domains {
                    for dom in domain_vec {
                        println!("☁ Domain: {}", dom.domainname,);

                        let domain_to_be_created = &MinimumDomain {
                            name: dom.domainname.clone(),
                            dns_txt_record: generate_txt_record(
                                dom.domainname.clone(),
                                d.id,
                                e.id.into(),
                            ),
                            certificate_id: dom.certificate_id.clone(),
                            environment_id: e.id.into(),
                            dns_zone: String::from("adobe.com."),
                        };

                        match create_singledomain(client, d.id, domain_to_be_created).await {
                            Ok(status) => match status {
                                StatusCode::OK => {
                                    println!("{:>8} Success", "✔");
                                }
                                _ => {
                                    eprintln!(
                                        "{:>8} {}",
                                        "Warning, check output above".yellow(),
                                        "⚠".yellow()
                                    );
                                    ret_value += 1;
                                }
                            },
                            Err(error) => {
                                eprintln!("{} {}", "❌ API error: ".red().bold(), error);
                                process::exit(1);
                            }
                        }
                    }
                }
            }
        }
    }
    if ret_value == 0 {
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::NOT_MODIFIED)
    }
}

async fn create_singledomain(
    client: &mut CloudManagerClient,
    program_id: u32,
    domain: &MinimumDomain,
) -> Result<StatusCode, Error> {
    let request_path = format!("{}/api/program/{}/domainNames", HOST_NAME, program_id);

    let response = client
        .perform_request(Method::POST, request_path, Some(domain), None)
        .await?;
    let status_code = response.status();
    let response_text = response.text().await?;
    if status_code != StatusCode::CREATED {
        let create_domain_response: CreateDomainResponse =
            serde_json::from_str(response_text.as_str()).unwrap_or_else(|_| {
                throw_adobe_api_error(response_text.clone());
                process::exit(1);
            });
        if let Some(error_vec) = &create_domain_response.errors {
            for error in error_vec {
                eprintln!(
                    "{:>8} Warning {} not created, Reason: {}",
                    "⚠", domain.name, error.code
                );
            }
            Ok(StatusCode::CONFLICT)
        } else {
            eprintln!("Created: {}", domain.name);
            Ok(StatusCode::OK)
        }
    } else {
        Ok(StatusCode::OK)
    }
}
/// Generates a txt record for adobe domain verification.
///
/// # Arguments
///
/// * `domain` - String of the domain
/// * `program_id` - A u32 that holds the program ID
/// * `env_id` - A i64 that holds the environment ID
/// * returns a String
/// ```
fn generate_txt_record(domain: String, program_id: u32, env_id: i64) -> String {
    let uuid = Uuid::new_v4();
    // adobe-aem-verification=<domain-name>/<program-id>/<environment-id>/<random-8-4-4-4-12-guid>\
    let txt_record = format!(
        "adobe-aem-verification={}/{}/{}/{}",
        &domain,
        &program_id,
        &env_id,
        &uuid.hyphenated().to_string(),
    );
    txt_record
}
