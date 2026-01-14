use x509_parser::prelude::FromDer;
use crate::client::{AdobeConnector, CloudManagerClient};
use crate::errors::throw_adobe_api_error;
use crate::models::certificates::{Certificate, CertificateList, CertificateResponse};
use crate::models::config::{CertificateConfig, ProgramsConfig, YamlConfig};
use crate::models::domain::MinimumDomain;
use crate::HOST_NAME;
use anyhow::{Context, Result};
use colored::Colorize;
use reqwest::{Error, Method, StatusCode};
use std::any::Any;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str;
use std::{fs, io, process};
use x509_parser::prelude::{Pem, X509Certificate}; // X509Certificate, etc.
use time::OffsetDateTime;



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
        .perform_request(Method::GET, request_path, None::<()>, Some(query_parameters))
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
    let mut ret_value = 0;

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

    // 3) Preflight: collect *all* missing files across the entire YAML
    match collect_all_missing_in_config(&base_dir, &config) {
        Ok(all_missing) => {
            if !all_missing.is_empty() {
                eprintln!(
                    "{}",
                    "❌ Preflight failed: missing certificate files found"
                        .red()
                        .bold()
                );
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

    // get current certificate
    let existing_certificates = get_certificates(client, program_id, &0, &1000)
        .await
        .unwrap();

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

                println!(
                    "☁ Manage certificate: {} ({:#?})",
                    cert_cfg.name, cert_cfg.id
                );

                println!("      certificate : {}, serial: {}", cert_cfg.certificate, meta.serial_dec);
                println!("      chain       : {}", cert_cfg.chain);
                println!("      key         : {}", cert_cfg.key);

                println!("      certificate : {}", cert_path.display());
                println!("      chain       : {}", chain_path.display());
                println!("      key         : {}", key_path.display());

                let found: Option<&Certificate> = find_existing_by_id_or_name(&existing_certificates.list, cert_cfg.id, &cert_cfg.name);

                if let Some(existing_cert) = found {
                    println!("      found existing certificate");
                    println!("          name         : {}", existing_cert.name);
                    println!("          id           : {}", existing_cert.id);

                    // update
                    println!("          serial_number: {}", existing_cert.serial_number);
                } else {
                    println!("      no matching existing certificate found, create new certificate!");
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
pub struct CertMeta {
    pub path: PathBuf,
    pub serial_hex: String,        // uppercase, no colons, no leading zeros
    pub serial_hex_colon: String,  // upper, colon-separated pairs
    pub serial_dec: String,        // decimal string
    pub not_before: OffsetDateTime,
    pub not_after: OffsetDateTime,
}

/// Read first CERTIFICATE from a file (PEM bundle or DER), extract serial + validity.

pub fn read_cert_meta(path: &Path) -> Result<CertMeta, io::Error> {
    let data = fs::read(path).map_err(|e| {
        io::Error::new(e.kind(), format!("Failed to read certificate ({}): {}", path.display(), e))
    })?;

    // Try PEM first: iterate over all PEM blocks, pick the first CERTIFICATE
    let mut pem_iter = Pem::iter_from_buffer(&data);
    while let Some(item) = pem_iter.next() {
        let pem = item.map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("PEM parse error: {e}"))
        })?;
        if pem.label == "CERTIFICATE" {
            let (_, cert) = X509Certificate::from_der(&pem.contents).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("Invalid DER in PEM: {e}"))
            })?;
            return extract_meta_from_cert(path.to_path_buf(), &cert);
        }
    }

    // If no PEM CERTIFICATE block found, try DER directly
    let (_, cert) = X509Certificate::from_der(&data).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("Invalid DER X.509: {e}"))
    })?;
    extract_meta_from_cert(path.to_path_buf(), &cert)
}



fn extract_meta_from_cert(path: PathBuf, cert: &X509Certificate<'_>) -> Result<CertMeta, io::Error> {
    let raw = cert.tbs_certificate.raw_serial();
    let raw_no_leading_zero = if raw.first() == Some(&0x00) { &raw[1..] } else { raw };

    let serial_hex = to_hex_no_colon_upper(raw_no_leading_zero);
    let serial_hex_colon = to_hex_colon_upper(raw_no_leading_zero);
    let serial_dec = big_endian_bytes_to_decimal(raw_no_leading_zero);

    let nb = cert.validity().not_before.to_datetime();
    let na = cert.validity().not_after.to_datetime();

    Ok(CertMeta { path, serial_hex, serial_hex_colon, serial_dec, not_before: nb, not_after: na })
}

fn to_hex_no_colon_upper(bytes: &[u8]) -> String {
    if bytes.is_empty() { return "0".to_string(); }
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes { use std::fmt::Write; let _ = write!(s, "{:02X}", b); }
    let t = s.trim_start_matches('0');
    if t.is_empty() { "0".into() } else { t.into() }
}

fn to_hex_colon_upper(bytes: &[u8]) -> String {
    let hex = to_hex_no_colon_upper(bytes);
    let padded = if hex.len() % 2 == 1 { format!("0{hex}") } else { hex };
    padded.as_bytes().chunks(2)
        .map(std::str::from_utf8)
        .filter_map(Result::ok)
        .collect::<Vec<_>>()
        .join(":")
}

fn big_endian_bytes_to_decimal(bytes: &[u8]) -> String {
    if bytes.is_empty() { return "0".into(); }
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
