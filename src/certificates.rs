use crate::client::{AdobeConnector, CloudManagerClient};
use crate::errors::throw_adobe_api_error;
use crate::models::certificates::{CertificateList, CertificateResponse};
use crate::models::config::{CertificateConfig, ProgramsConfig, YamlConfig};
use crate::models::domain::MinimumDomain;
use crate::HOST_NAME;
use colored::Colorize;
use reqwest::{Error, Method, StatusCode};
use std::{fs, io, process};
use std::any::Any;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str;
use anyhow::{Context, Result};

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
        .perform_request(Method::GET, request_path, None::<()>, None)
        .await?
        .text()
        .await?;
    let certificates: CertificateResponse =
        serde_json::from_str(response.as_str()).unwrap_or_else(|_| {
            throw_adobe_api_error(response);
            process::exit(1);
        });

    Ok(certificates.certificate_list)
}

/// Manages a list of certificates.
///
/// Manage currently only supports creating/updating an existing certificate with a new one
///
/// # Arguments
///
/// * `file_path` - String slice that holds the path to the YAML variables config
/// * `client` - A mutable reference to a CloudManagerClient instance
pub async fn manage_certificates(
    file_path: String,
    client: &mut CloudManagerClient,
) -> anyhow::Result<StatusCode> {

    let mut ret_value = 0;

    // 1) Load YAML as you already do
    let config: YamlConfig = YamlConfig::from_file(file_path.clone());
    let programs: &Vec<ProgramsConfig> = &config.programs;

    // 2) Derive base_dir from the YAML file path
    let yaml_path = Path::new(&file_path);
    let base_dir = match base_dir_from_yaml_path(yaml_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} {}", "❌ Failed to derive base dir from YAML path:".red(), e);
            std::process::exit(1);
        }
    };

    // 3) Preflight: collect *all* missing files across the entire YAML
    match collect_all_missing_in_config(&base_dir, &config) {
        Ok(all_missing) => {
            if !all_missing.is_empty() {
                eprintln!("{}", "❌ Preflight failed: missing certificate files found".red().bold());
                for m in all_missing {
                    eprintln!("  - {}", m);
                }
                eprintln!(
                    "\n{}",
                    "Please fix the file paths above or move the files into place. \
                     Paths are resolved relative to the YAML file's directory."
                        .yellow()
                );
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{} {}", "❌ Preflight failed:".red(), e);
            std::process::exit(1);
        }
    }

    // manage certificates
    for program in programs {
        println!("☁ Program: {}", program.id,);


        if let Some(certs) = &program.certificates {
            for cert_cfg in certs {

                println!("☁ Manage certificate: {} ({:#?})", cert_cfg.name, cert_cfg.id);

                println!("      certificate : {}", cert_cfg.certificate);
                println!("      chain       : {}", cert_cfg.chain);
                println!("      key         : {}", cert_cfg.key);


            }
        }
    }

    if ret_value == 0 {
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::NOT_MODIFIED)
    }
}


/// Structured return that contains absolute paths and the loaded contents.
#[derive(Debug)]
pub struct LoadedCerts {
    pub certificate_path: PathBuf,
    pub chain_path: PathBuf,
    pub key_path: PathBuf,
    pub certificate: String, // PEM text; switch to Vec<u8> + fs::read for DER/binary
    pub chain: String,
    pub key: String,
}



// If not already present in your module:
fn resolve_against_base<P: AsRef<Path>>(base_dir: &Path, p: P) -> PathBuf {
    let p = p.as_ref();
    if p.is_absolute() { p.to_path_buf() } else { base_dir.join(p) }
}

fn absolutize_for_errors(p: &Path) -> io::Result<PathBuf> {
    if p.is_absolute() {
        Ok(p.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(p))
    }
}

/// Derive the base directory from the YAML file path used to load config.
/// If the path has no parent, fall back to current_dir().
pub fn base_dir_from_yaml_path(yaml_path: &Path) -> io::Result<PathBuf> {
    if let Some(parent) = yaml_path.parent() {
        if !parent.as_os_str().is_empty() {
            return Ok(parent.to_path_buf());
        }
    }
    std::env::current_dir()
}

/// Preflight for *one* certificate: return a list of missing file messages (absolute paths).
pub fn collect_missing_cert_paths(base_dir: &Path, cfg: &CertificateConfig) -> io::Result<Vec<String>> {
    let cert_path  = absolutize_for_errors(&resolve_against_base(base_dir, &cfg.certificate))?;
    let chain_path = absolutize_for_errors(&resolve_against_base(base_dir, &cfg.chain))?;
    let key_path   = absolutize_for_errors(&resolve_against_base(base_dir, &cfg.key))?;

    let mut missing = Vec::new();
    if !cert_path.exists()  { missing.push(format!("certificate file is missing: {}", cert_path.display())); }
    if !chain_path.exists() { missing.push(format!("chain file is missing: {}", chain_path.display())); }
    if !key_path.exists()   { missing.push(format!("key file is missing: {}", key_path.display())); }
    Ok(missing)
}

/// Preflight across the *entire* YAML config (all programs / all certificates).
/// Returns a flat list of human-readable messages with full context and absolute paths.
/// If the vector is empty, everything exists.
pub fn collect_all_missing_in_config(base_dir: &Path, config: &YamlConfig) -> io::Result<Vec<String>> {
    let mut all_missing = Vec::new();

    for program in &config.programs {
        if let Some(certs) = &program.certificates {
            for cert_cfg in certs {
                let missing = collect_missing_cert_paths(base_dir, cert_cfg)?;
                if !missing.is_empty() {
                    for msg in missing {
                        // tag each message with program/cert context
                        all_missing.push(format!(
                            "program {} / cert '{}': {}",
                            program.id, cert_cfg.name, msg
                        ));
                    }
                }
            }
        }
    }

    Ok(all_missing)
}




