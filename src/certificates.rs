use crate::client::{AdobeConnector, CloudManagerClient};
use crate::errors::throw_adobe_api_error;
use crate::models::certificates::{Certificate, CertificateList, CertificateResponse, CreateUpdateCertificate, CreateUpdateCertificateResponse, StringValue};
use crate::models::config::{CertificateConfig, ProgramsConfig, YamlConfig};
use crate::HOST_NAME;
use anyhow::{anyhow, Result};
use colored::Colorize;
use reqwest::{Error, Method, StatusCode};
use std::path::{Path, PathBuf};
use std::str;
use std::{fs, io, process};
use time::OffsetDateTime;
use x509_parser::prelude::FromDer;
use x509_parser::prelude::{Pem, X509Certificate}; // X509Certificate, etc.

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
            Some(query_parameters),
        )
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
    program_id: u32,
    client: &mut CloudManagerClient,
) -> anyhow::Result<StatusCode> {
    let mut certs_updated: Vec<&CertificateConfig> = Vec::new();
    let mut certs_created: Vec<&CertificateConfig> = Vec::new();
    let mut certs_skipped: Vec<&CertificateConfig> = Vec::new();
    let mut certs_failed: Vec<&CertificateConfig> = Vec::new();

    // 1) Load YAML as you already do
    let config: YamlConfig = YamlConfig::from_file(file_path.clone());
    let programs: &Vec<ProgramsConfig> = &config.programs;

    // 2) Derive base_dir from the YAML file path
    let yaml_path = Path::new(&file_path);
    let base_dir = match base_dir_from_yaml_path(yaml_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "{} {}",
                "❌ Failed to derive base dir from YAML path:".red(),
                e
            );
            std::process::exit(1);
        }
    };

    println!("🚀 Preflight check - check if all refrenced certificate files are available...");

    // 3) Preflight: collect *all* missing files across the entire YAML
    match collect_all_missing_in_config(&base_dir, &config) {
        Ok(all_missing) => {
            if !all_missing.is_empty() {
                eprintln!(
                    "{}  {}",
                    "❌",
                    "Preflight check failed: missing certificate files found"
                        .red()
                        .bold()
                );
                for m in all_missing {
                    eprintln!("{:>4} {}", "🚧", m);
                }
                eprintln!(
                    "\n{}",
                    "Please fix the file paths above or move the files into place. \
                     Paths are resolved relative to the YAML file's directory."
                        .yellow()
                );
                std::process::exit(1);
            } else {
                println!("{} {}", "🎉", "Preflight check succcessful.");
            }
        }
        Err(e) => {
            eprintln!("{} {} {}", "❌", "Preflight check failed:".red(), e);
            std::process::exit(1);
        }
    }

    // get current certificate
    let existing_certificates = get_certificates(client, program_id, &0, &1000)
        .await
        .unwrap();

    println!();

    // manage certificates
    for program in programs {
        println!("☁ Program: {}", program.id,);

        if let Some(certs) = &program.certificates {
            for cert_cfg in certs {
                let cert_path =
                    absolutize_for_errors(&resolve_against_base(&base_dir, &cert_cfg.certificate))?;

                let meta = read_cert_meta(&cert_path).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("failed to parse certificate '{}': {e}", cert_path.display()),
                    )
                })?;

                let chain_path =
                    absolutize_for_errors(&resolve_against_base(&base_dir, &cert_cfg.chain))?;
                let key_path =
                    absolutize_for_errors(&resolve_against_base(&base_dir, &cert_cfg.key))?;

                println!("{:>4} Manage certificate: {}", "🏅", cert_cfg.name);
                println!("{:>6} id         : {:?}", "🆔" , cert_cfg.id);
                println!("{:>6} serial     : {}", "🔢" , meta.serial_dec);
                println!("{:>6} not before : {}", "📆" , meta.not_before);
                println!("{:>6} not after  : {}", "⏳ " , meta.not_after);
                println!("{:>6} certificate: {}", "📜" , cert_cfg.certificate);
                println!("{:>6} chain      : {}", "🔗" , cert_cfg.chain);
                println!("{:>6} key        : {}", "🔑" , cert_cfg.key);
                println!();
                println!("{:>8} check for existing certificate", "🔎");

                let found_existing_cert: Option<&Certificate> = find_existing_by_id_or_name(
                    &existing_certificates.list,
                    cert_cfg.id,
                    &cert_cfg.name,
                );

                let (certificate_pem, chain_pem, key_pem) = load_cert_files(&cert_path, &chain_path, &key_path)
                    .map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("Failed to read cert files for '{}': {}", cert_cfg.name, e),
                        )
                    })?;

                let mut certificate_action = CertificateAction::SKIP;

                let mut new_cert: CreateUpdateCertificate = CreateUpdateCertificate {
                    id: None,
                    name: cert_cfg.name.clone(),
                    certificate: certificate_pem,
                    chain: chain_pem,
                    private_key : StringValue {
                        value: key_pem
                    },
                };

                if let Some(existing_cert) = found_existing_cert {
                    new_cert.id = Some(existing_cert.id);

                    if existing_cert.serial_number != meta.serial_dec {
                        println!("{:>8} existing certificate found and serial number is different:", "🔦");
                        certificate_action = CertificateAction::UPDATE;

                    } else {
                        println!("{:>8} existing certificate found and serial number matches:", "🔦");
                        certs_skipped.push(cert_cfg);
                    }

                    println!("{:>12} existing: {}", "🔢", existing_cert.serial_number);
                    println!("{:>12} new     : {}", "🔢", meta.serial_dec);
                } else {
                    certificate_action = CertificateAction::CREATE;
                }

                println!("{:>8} action: {:?} ", "🔨", certificate_action);

                if certificate_action == CertificateAction::CREATE || certificate_action == CertificateAction::UPDATE {
                    let result = perform_create_update(&new_cert, program_id, client).await?;
                    if result != StatusCode::OK {
                        certs_failed.push(cert_cfg);
                    } else {
                        if certificate_action == CertificateAction::CREATE {
                            certs_created.push(cert_cfg);
                        } else {
                            certs_updated.push(cert_cfg);
                        }
                    }
                }

                println!();
            }

        }

    }
    println!("\n🚀 Management of certificates complete.\n");

    println!("{:>12} {}","Skipped:", certs_skipped.len());
    println!("{:>12} {}","Updated:", certs_updated.len());
    println!("{:>12} {}","Created:", certs_created.len());
    println!("{:>12} {}","Failed:", certs_failed.len());
    println!("\n");

    if !certs_failed.is_empty() {
        eprintln!("{}  {}", "❌", "Issues found, please check logs".red().bold());
        Err(anyhow!(
        "Failure during creating/updating certificates, check logs for details"
        ))

    } else {
        println!("{} {}", "🎉", "No issues found.");
        Ok(StatusCode::OK)
    }

}

async fn perform_create_update(cert: &CreateUpdateCertificate, program_id: u32, client: &mut CloudManagerClient) -> core::result::Result<StatusCode, Error> {
    let request_path = format!("{}/api/program/{}/certificates", HOST_NAME, program_id);

    let response = client
        .perform_request(Method::POST, request_path, Some(cert), None)
        .await?;
    let status_code = response.status();
    let response_text = response.text().await?;

    if status_code != StatusCode::CREATED {
        let create_certificate_response: CreateUpdateCertificateResponse =
            serde_json::from_str(response_text.as_str()).unwrap_or_else(|_| {
                throw_adobe_api_error(response_text.clone());
                process::exit(1);
            });

        eprintln!(
            "{:>8}  {} {}",
            "❌", cert.name, "not created/updated!".red().bold()
        );

        if let Some(additional_properties) = &create_certificate_response.additional_properties {
            if let Some(error_vec) = &additional_properties.errors {

                for error in error_vec {
                    eprintln!(
                        "{:>12} {} {}",
                        "⚠️", "Field:".yellow().bold(), error.field
                    );
                    eprintln!(
                        "{:>19} {}",
                        "Code:".yellow().bold(), error.code
                    );
                    eprintln!(
                        "{:>19} {}",
                        "Message:".yellow().bold(), error.message
                    );
                }

            }
            Ok(StatusCode::NOT_ACCEPTABLE)
        } else {
            eprintln!("{:>8}  {} {}",
                      "❌", "Unknown error while creating".red().bold(), cert.name);
            eprintln!("{:>18} {}","reponse: ".red(), response_text);
            Ok(StatusCode::NOT_ACCEPTABLE)
        }
    } else {
        println!(
            "{:>8}  Certificate {} {}",
            "✅", cert.name, "created/updated.".green().bold()
        );
        Ok(StatusCode::OK)
    }
}

fn load_cert_files(cert_path: &PathBuf, chain_path: &PathBuf, key_path: &PathBuf) -> Result<(String, String, String), io::Error> {
    let certificate = fs::read_to_string(cert_path)?.replace("\n", "");
    let chain = fs::read_to_string(chain_path)?.replace("\n", "");
    let key = fs::read_to_string(key_path)?.replace("\n", "");
    Ok((certificate, chain, key))
}


fn find_existing_by_id_or_name<'a>(
    list: &'a [Certificate],
    yaml_id: Option<i64>,
    yaml_name: &str,
) -> Option<&'a Certificate> {
    if let Some(id_val) = yaml_id {
        list.iter().find(|ec| ec.id == id_val)
    } else {
        list.iter().find(|ec| ec.name == yaml_name)
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum CertificateAction {
    CREATE,
    UPDATE,
    SKIP,
}
#[derive(Debug)]
pub struct CertMeta {
    pub serial_dec: String,       // decimal string
    pub not_before: OffsetDateTime,
    pub not_after: OffsetDateTime,
}

/// Read first CERTIFICATE from a file (PEM bundle or DER), extract serial + validity.

pub fn read_cert_meta(path: &Path) -> Result<CertMeta, io::Error> {
    let data = fs::read(path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("Failed to read certificate ({}): {}", path.display(), e),
        )
    })?;

    // Try PEM first: iterate over all PEM blocks, pick the first CERTIFICATE
    let mut pem_iter = Pem::iter_from_buffer(&data);
    while let Some(item) = pem_iter.next() {
        let pem = item.map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("PEM parse error: {e}"))
        })?;
        if pem.label == "CERTIFICATE" {
            let (_, cert) = X509Certificate::from_der(&pem.contents).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid DER in PEM: {e}"),
                )
            })?;
            return extract_meta_from_cert(&cert);
        }
    }

    // If no PEM CERTIFICATE block found, try DER directly
    let (_, cert) = X509Certificate::from_der(&data).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid DER X.509: {e}"),
        )
    })?;
    extract_meta_from_cert(&cert)
}

fn extract_meta_from_cert(
    cert: &X509Certificate<'_>,
) -> Result<CertMeta, io::Error> {
    let raw = cert.tbs_certificate.raw_serial();
    let raw_no_leading_zero = if raw.first() == Some(&0x00) {
        &raw[1..]
    } else {
        raw
    };

    let serial_dec = big_endian_bytes_to_decimal(raw_no_leading_zero);

    let nb = cert.validity().not_before.to_datetime();
    let na = cert.validity().not_after.to_datetime();

    Ok(CertMeta {
        serial_dec,
        not_before: nb,
        not_after: na,
    })
}

fn big_endian_bytes_to_decimal(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "0".into();
    }
    let mut digits = vec![0u8];
    for &b in bytes {
        let mut carry = b as u16;
        for d in digits.iter_mut() {
            let v = (*d as u16) * 256 + carry;
            *d = (v % 10) as u8;
            carry = v / 10;
        }
        while carry > 0 {
            digits.push((carry % 10) as u8);
            carry /= 10;
        }
    }
    digits.iter().rev().map(|d| (b'0' + *d) as char).collect()
}

// If not already present in your module:
fn resolve_against_base<P: AsRef<Path>>(base_dir: &Path, p: P) -> PathBuf {
    let p = p.as_ref();
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base_dir.join(p)
    }
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
pub fn collect_missing_cert_paths(
    base_dir: &Path,
    cfg: &CertificateConfig,
) -> io::Result<Vec<String>> {
    let cert_path = absolutize_for_errors(&resolve_against_base(base_dir, &cfg.certificate))?;
    let chain_path = absolutize_for_errors(&resolve_against_base(base_dir, &cfg.chain))?;
    let key_path = absolutize_for_errors(&resolve_against_base(base_dir, &cfg.key))?;

    let mut missing = Vec::new();
    if !cert_path.exists() {
        missing.push(format!(
            "certificate file is missing: {}",
            cert_path.display()
        ));
    }
    if !chain_path.exists() {
        missing.push(format!("chain file is missing: {}", chain_path.display()));
    }
    if !key_path.exists() {
        missing.push(format!("key file is missing: {}", key_path.display()));
    }
    Ok(missing)
}

/// Preflight across the *entire* YAML config (all programs / all certificates).
/// Returns a flat list of human-readable messages with full context and absolute paths.
/// If the vector is empty, everything exists.
pub fn collect_all_missing_in_config(
    base_dir: &Path,
    config: &YamlConfig,
) -> io::Result<Vec<String>> {
    let mut all_missing = Vec::new();

    for program in &config.programs {
        if let Some(certs) = &program.certificates {
            for cert_cfg in certs {
                let missing = collect_missing_cert_paths(base_dir, cert_cfg)?;
                if !missing.is_empty() {
                    for msg in missing {
                        // tag each message with program/cert context
                        all_missing.push(format!(
                            "program {} - cert '{}': {}",
                            program.id, cert_cfg.name, msg
                        ));
                    }
                }
            }
        }
    }

    Ok(all_missing)
}
