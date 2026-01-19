use crate::client::{AdobeConnector, CloudManagerClient};
use crate::errors::throw_adobe_api_error;
use crate::models::certificates::{
    Certificate, CertificateList, CertificateResponse, CreateUpdateCertificate,
    CreateUpdateCertificateResponse, StringValue,
};
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

/// Retrieves a list of certificates for a given program from the Cloud Manager API.
///
/// This function performs an HTTP `GET` request against the
/// `/api/program/{program_id}/certificates` endpoint and returns a paginated
/// list of certificates.
///
/// Pagination is controlled via the `start` and `limit` query parameters, which
/// are forwarded directly to the API.
///
/// # Parameters
/// * `client` – A mutable `CloudManagerClient` used to execute the HTTP request.
/// * `program_id` – The ID of the program whose certificates should be fetched.
/// * `start` – The zero‑based index of the first certificate to retrieve.
/// * `limit` – The maximum number of certificates to return.
///
/// # Returns
/// * `Ok(CertificateList)` containing the list of certificates returned by the API.
/// * `Err(Error)` if the request fails or the response cannot be read.
///
/// # Errors
/// This function may fail in the following situations:
///
/// * Network or transport errors during the HTTP request
/// * The API response body cannot be read
/// * The API returns invalid or unexpected JSON
///
/// If JSON deserialization fails, the raw Adobe API error is emitted and the
/// process terminates.
///
/// # Notes
/// * The function assumes that the API response conforms to the
///   `CertificateResponse` schema.
/// * No retry or pagination logic is implemented; callers must handle paging.
/// * A fatal deserialization error causes an immediate process exit.
/// * The `start` and `limit` parameters are passed verbatim and are not validated.
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

/// Manages the lifecycle of certificates defined in a YAML configuration file.
///
/// This function orchestrates the full certificate management workflow:
///
/// 1. Loads and parses a YAML configuration file.
/// 2. Performs a preflight check to ensure all referenced certificate files exist.
/// 3. Fetches existing certificates from the Cloud Manager API.
/// 4. Compares local certificates with existing ones based on serial numbers.
/// 5. Creates, updates, or skips certificates as needed.
/// 6. Prints a summarized result of all operations.
///
/// The function is designed to be used by a CLI and performs user‑facing output
/// throughout the process.
///
/// # Parameters
/// * `file_path` – Path to the YAML configuration file.
/// * `program_id` – The Cloud Manager program ID under which certificates
///   are managed.
/// * `client` – A mutable `CloudManagerClient` used to communicate with the API.
///
/// # Returns
/// * `Ok(StatusCode::OK)` if all certificates were processed successfully and
///   no errors occurred.
/// * `Err(anyhow::Error)` if one or more certificates failed to be created or
///   updated, or if a fatal error occurred during processing.
///
/// # Errors
/// This function may return or trigger errors in the following situations:
///
/// * The YAML configuration file cannot be parsed.
/// * One or more referenced certificate files are missing.
/// * Certificate parsing or metadata extraction fails.
/// * API communication or certificate creation/update fails.
///
/// Some fatal errors (such as preflight validation failures) terminate the
/// process early with `std::process::exit`.
///
/// # Workflow Overview
///
/// ```text
/// YAML file
///    ↓
/// Preflight validation (file existence)
///    ↓
/// Fetch existing certificates
///    ↓
/// For each configured certificate:
///    ├─ Read certificate metadata
///    ├─ Compare with existing certificates
///    ├─ Decide CREATE / UPDATE / SKIP
///    └─ Execute API call if required
///    ↓
/// Print summary and final status
/// ```
///
/// # Notes
/// * Certificate matching is done by ID (if present) or by name.
/// * Updates are determined by comparing certificate serial numbers.
/// * Only one certificate per configuration entry is processed.

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
                println!("{:>6} id         : {:?}", "🆔", cert_cfg.id);
                println!("{:>6} serial     : {}", "🔢", meta.serial_dec);
                println!("{:>6} not before : {}", "📆", meta.not_before);
                println!("{:>6} not after  : {}", "⏳ ", meta.not_after);
                println!("{:>6} certificate: {}", "📜", cert_cfg.certificate);
                println!("{:>6} chain      : {}", "🔗", cert_cfg.chain);
                println!("{:>6} key        : {}", "🔑", cert_cfg.key);
                println!();
                println!("{:>8} check for existing certificate", "🔎");

                let found_existing_cert: Option<&Certificate> = find_existing_by_id_or_name(
                    &existing_certificates.list,
                    cert_cfg.id,
                    &cert_cfg.name,
                );

                let (certificate_pem, chain_pem, key_pem) =
                    load_cert_files(&cert_path, &chain_path, &key_path).map_err(|e| {
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
                    private_key: StringValue { value: key_pem },
                };

                if let Some(existing_cert) = found_existing_cert {
                    new_cert.id = Some(existing_cert.id);

                    if existing_cert.serial_number != meta.serial_dec {
                        println!(
                            "{:>8} existing certificate found and serial number is different:",
                            "🔦"
                        );
                        certificate_action = CertificateAction::UPDATE;
                    } else {
                        println!(
                            "{:>8} existing certificate found and serial number matches:",
                            "🔦"
                        );
                        certs_skipped.push(cert_cfg);
                    }

                    println!("{:>12} existing: {}", "🔢", existing_cert.serial_number);
                    println!("{:>12} new     : {}", "🔢", meta.serial_dec);
                } else {
                    certificate_action = CertificateAction::CREATE;
                }

                println!("{:>8} action: {:?} ", "🔨", certificate_action);

                if certificate_action == CertificateAction::CREATE
                    || certificate_action == CertificateAction::UPDATE
                {
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

    println!("{:>12} {}", "Skipped:", certs_skipped.len());
    println!("{:>12} {}", "Updated:", certs_updated.len());
    println!("{:>12} {}", "Created:", certs_created.len());
    println!("{:>12} {}", "Failed:", certs_failed.len());
    println!("\n");

    if !certs_failed.is_empty() {
        eprintln!(
            "{}  {}",
            "❌",
            "Issues found, please check logs".red().bold()
        );
        Err(anyhow!(
            "Failure during creating/updating certificates, check logs for details"
        ))
    } else {
        println!("{} {}", "🎉", "No issues found.");
        Ok(StatusCode::OK)
    }
}

/// Creates or updates a certificate for a given program via the Cloud Manager API.
///
/// This function sends a POST request to the
/// `/api/program/{program_id}/certificates` endpoint. Depending on the API
/// response, it prints success or detailed error information to the console.
///
/// # Parameters
/// - `cert`: The certificate payload used for creation or update.
/// - `program_id`: The ID of the program the certificate belongs to.
/// - `client`: A mutable CloudManagerClient used to perform the HTTP request.
///
/// # Returns
/// - `Ok(StatusCode::OK)` if the certificate was successfully created or updated.
/// - `Ok(StatusCode::NOT_ACCEPTABLE)` if the API returned a validation or logical error.
/// - `Err(Error)` if the request itself failed (e.g. network errors).
///
/// # Notes
/// - On JSON deserialization failure of an error response, the function will
///   emit an Adobe API error and terminate the process.
async fn perform_create_update(
    cert: &CreateUpdateCertificate,
    program_id: u32,
    client: &mut CloudManagerClient,
) -> core::result::Result<StatusCode, Error> {
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
            "❌",
            cert.name,
            "not created/updated!".red().bold()
        );

        if let Some(additional_properties) = &create_certificate_response.additional_properties {
            if let Some(error_vec) = &additional_properties.errors {
                for error in error_vec {
                    eprintln!("{:>12} {} {}", "⚠️", "Field:".yellow().bold(), error.field);
                    eprintln!("{:>19} {}", "Code:".yellow().bold(), error.code);
                    eprintln!("{:>19} {}", "Message:".yellow().bold(), error.message);
                }
            }
            Ok(StatusCode::NOT_ACCEPTABLE)
        } else {
            eprintln!(
                "{:>8}  {} {}",
                "❌",
                "Unknown error while creating".red().bold(),
                cert.name
            );
            eprintln!("{:>18} {}", "reponse: ".red(), response_text);
            Ok(StatusCode::NOT_ACCEPTABLE)
        }
    } else {
        println!(
            "{:>8}  Certificate {} {}",
            "✅",
            cert.name,
            "created/updated.".green().bold()
        );
        Ok(StatusCode::OK)
    }
}

/// Loads certificate-related files from disk and returns their contents as strings.
///
/// This function reads three files:
/// - the certificate file
/// - the certificate chain file
/// - the private key file
///
/// All files are read as UTF‑8 text. Newline characters (`\n`) are removed from
/// each file’s contents before returning the result, producing single‑line
/// strings suitable for API transmission or embedding in JSON payloads.
///
/// # Parameters
/// - `cert_path`: Path to the certificate file (e.g. `cert.pem`).
/// - `chain_path`: Path to the certificate chain file (e.g. `chain.pem`).
/// - `key_path`: Path to the private key file (e.g. `private.key`).
///
/// # Returns
/// - `Ok((certificate, chain, key))` containing the contents of all three files
///   as newline‑free `String`s, in the order:
///   `(certificate, chain, key)`.
/// - `Err(io::Error)` if any of the files cannot be read.
///
/// # Errors
/// This function returns an `io::Error` in the following cases:
/// - One or more files do not exist
/// - Insufficient file permissions
/// - The file contents are not valid UTF‑8
///
/// # Notes
/// - All newline characters are removed unconditionally.
/// - No validation of the certificate or key contents is performed.
fn load_cert_files(
    cert_path: &PathBuf,
    chain_path: &PathBuf,
    key_path: &PathBuf,
) -> Result<(String, String, String), io::Error> {
    let certificate = fs::read_to_string(cert_path)?.replace("\n", "");
    let chain = fs::read_to_string(chain_path)?.replace("\n", "");
    let key = fs::read_to_string(key_path)?.replace("\n", "");
    Ok((certificate, chain, key))
}

/// Finds an existing certificate by ID or, if no ID is provided, by name.
///
/// This helper function searches through a slice of `Certificate` objects and
/// returns a reference to the first matching entry. The lookup strategy depends
/// on the provided parameters:
///
/// * If `yaml_id` is `Some`, the function searches for a certificate with a
///   matching `id`.
/// * If `yaml_id` is `None`, the function falls back to searching by `name`
///   using `yaml_name`.
///
/// # Parameters
/// * `list` – A slice of existing certificates to search.
/// * `yaml_id` – An optional certificate ID, typically provided via configuration
///   (e.g. a YAML file). When present, it takes precedence over the name.
/// * `yaml_name` – The certificate name used as a fallback lookup key when no
///   ID is provided.
///
/// # Returns
/// * `Some(&Certificate)` – A reference to the first matching certificate found
///   in the list.
/// * `None` – If no certificate matches the given ID or name.
///
///
/// # Notes
/// * If both an ID and a name refer to different certificates, the ID always wins.
/// * Name matching is performed using strict equality and is case‑sensitive.
/// * The function stops searching as soon as a match is found.
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

/// Enum for performed action on a certificate
#[derive(Debug, PartialEq)]
pub enum CertificateAction {
    CREATE,
    UPDATE,
    SKIP,
}

/// Struct for holding certificate metadata
#[derive(Debug)]
pub struct CertMeta {
    pub serial_dec: String, // decimal string
    pub not_before: OffsetDateTime,
    pub not_after: OffsetDateTime,
}

/// Reads an X.509 certificate file and extracts its metadata.
///
/// This function loads a certificate from disk and attempts to parse it in
/// **PEM** format first. If no PEM `CERTIFICATE` block is found, it falls back
/// to parsing the file contents directly as **DER‑encoded** X.509 data.
///
/// Once a valid certificate is successfully parsed, the function extracts
/// selected metadata fields (such as serial number and validity period) by
/// delegating to [`extract_meta_from_cert`].
///
/// # Parameters
/// * `path` – Path to the certificate file. The file may be in PEM or DER format.
///
/// # Returns
/// * `Ok(CertMeta)` containing extracted metadata from the certificate.
/// * `Err(io::Error)` if the file cannot be read, parsed, or does not contain a
///   valid X.509 certificate.
///
/// # Errors
/// This function may return an `io::Error` in the following situations:
/// * The certificate file cannot be read from disk
/// * PEM parsing fails due to invalid formatting
/// * No PEM `CERTIFICATE` block is present and DER parsing fails
/// * The certificate contains invalid ASN.1 or DER data
///
/// All errors include contextual information such as the file path or parsing
/// failure reason to improve diagnostics.
///
/// # Parsing Strategy
/// 1. Read the file as raw bytes.
/// 2. Attempt to parse PEM blocks and select the first `CERTIFICATE` block.
/// 3. If no PEM certificate is found, attempt to parse the file as DER directly.
/// 4. Extract certificate metadata from the parsed X.509 structure.
///
/// # Notes
/// * Only the **first** PEM `CERTIFICATE` block is used if multiple blocks exist.
/// * No certificate chain validation or signature verification is performed.
/// * The function does not distinguish between end‑entity and CA certificates.
/// * The file extension is ignored; detection is based solely on content.

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

/// Extracts metadata from an X.509 certificate.
///
/// This function reads selected metadata fields from the **To‑Be‑Signed (TBS)**
/// section of an X.509 certificate:
///
/// * the certificate serial number (converted to a decimal string)
/// * the `notBefore` validity timestamp
/// * the `notAfter` validity timestamp
///
/// The serial number is returned without a leading zero byte if present.
/// This is necessary because X.509 serial numbers may be encoded as signed
/// integers in ASN.1, which can introduce a leading `0x00` byte.
///
/// # Parameters
/// * `cert` – A parsed `X509Certificate` reference from which metadata is extracted.
///
/// # Returns
/// * `Ok(CertMeta)` containing the extracted certificate metadata:
///   * `serial_dec` – The serial number represented as a decimal string
///   * `not_before` – The start of the certificate validity period
///   * `not_after` – The end of the certificate validity period
/// * `Err(io::Error)` if metadata extraction or conversion fails
///
/// # Errors
/// This function may return an `io::Error` if:
/// * Serial number conversion fails
/// * Date/time conversion fails
///
/// # Notes
/// * Leading zero bytes in the serial number are stripped before conversion.
/// * The returned timestamps are converted to `chrono::DateTime`.
/// * No validation of the certificate signature or trust chain is performed.
fn extract_meta_from_cert(cert: &X509Certificate<'_>) -> Result<CertMeta, io::Error> {
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

/// Converts a big‑endian byte slice representing an unsigned integer
/// into its decimal string representation.
///
/// This function interprets `bytes` as an **unsigned integer encoded in
/// big‑endian order** (most significant byte first) and converts it into
/// a base‑10 string without using big‑integer or arbitrary‑precision libraries.
///
/// The conversion is performed manually by repeatedly multiplying the
/// current decimal representation by 256 and adding the next byte.
///
/// # Parameters
/// * `bytes` – A slice of bytes representing an unsigned big‑endian integer.
///
/// # Returns
/// * A `String` containing the decimal representation of the input value.
/// * Returns `"0"` if the input slice is empty.
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

/// Resolves a path against a base directory if it is relative.
///
/// This helper function takes a base directory and a path and ensures that
/// the returned `PathBuf` is correctly resolved:
///
/// * If `p` is already an absolute path, it is returned unchanged.
/// * If `p` is a relative path, it is joined with `base_dir`.
///
/// This is useful when working with configuration files or user input that
/// may contain a mix of absolute and relative paths.
///
/// # Type Parameters
/// * `P` – Any type that can be referenced as a `Path` (e.g. `&Path`, `PathBuf`,
///   or `&str`).
///
/// # Parameters
/// * `base_dir` – The base directory used to resolve relative paths.
/// * `p` – The path to resolve, either absolute or relative.
///
/// # Returns
/// * A `PathBuf` containing the resolved path.
fn resolve_against_base<P: AsRef<Path>>(base_dir: &Path, p: P) -> PathBuf {
    let p = p.as_ref();
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base_dir.join(p)
    }
}

/// Converts a path into an absolute path for error reporting purposes.
///
/// This function ensures that the returned path is absolute, making it
/// suitable for use in error messages, logs, or diagnostics:
///
/// * If the input path is already absolute, it is returned unchanged.
/// * If the input path is relative, it is resolved against the current
///   working directory.
///
/// Unlike `resolve_against_base`, this function uses the process’s
/// current directory instead of a caller-provided base directory.
///
/// # Parameters
/// * `p` – The path to convert into an absolute path.
///
/// # Returns
/// * `Ok(PathBuf)` containing the absolute path.
/// * `Err(io::Error)` if the current working directory cannot be determined.
///
/// # Errors
/// This function fails if:
/// * The current working directory cannot be retrieved
///   (e.g. due to permission or filesystem errors).
///
fn absolutize_for_errors(p: &Path) -> io::Result<PathBuf> {
    if p.is_absolute() {
        Ok(p.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(p))
    }
}

/// Determines the base directory associated with a YAML configuration file path.
///
/// This function derives a base directory that can be used to resolve relative
/// paths referenced within a YAML configuration file:
///
/// * If `yaml_path` has a non‑empty parent directory, that directory is returned.
/// * If `yaml_path` has no parent (e.g. the file name is relative and located in
///   the current directory), the process’s current working directory is returned.
///
/// This logic ensures a sensible and consistent base directory for resolving
/// relative paths inside configuration files, regardless of how the YAML file
/// path itself was specified.
///
/// # Parameters
/// * `yaml_path` – The path to the YAML configuration file.
///
/// # Returns
/// * `Ok(PathBuf)` containing the resolved base directory.
/// * `Err(io::Error)` if the current working directory cannot be determined.
///
/// # Errors
/// This function may return an `io::Error` if:
/// * The current working directory cannot be retrieved (e.g. due to permission
///   or filesystem issues).
///
/// # Notes
/// * The returned path is not canonicalized.
/// * No filesystem access is performed beyond querying the current working
///   directory when needed.
/// * This function does not verify that the YAML file itself exists.

pub fn base_dir_from_yaml_path(yaml_path: &Path) -> io::Result<PathBuf> {
    if let Some(parent) = yaml_path.parent() {
        if !parent.as_os_str().is_empty() {
            return Ok(parent.to_path_buf());
        }
    }
    std::env::current_dir()
}

/// Collects missing certificate‑related file paths referenced by a certificate
/// configuration.
///
/// This function resolves the certificate, chain, and private key paths from the
/// given `CertificateConfig` against a base directory, converts them into absolute
/// paths, and checks whether the referenced files exist on disk.
///
/// Any missing files are reported as human‑readable error messages that include
/// the resolved absolute file path, making them suitable for diagnostics and
/// CLI output.
///
/// # Parameters
/// * `base_dir` – The base directory used to resolve relative paths found in the
///   certificate configuration.
/// * `cfg` – The certificate configuration containing paths to the certificate,
///   chain, and private key files.
///
/// # Returns
/// * `Ok(Vec<String>)` containing zero or more error messages describing missing
///   files.
///   * An empty vector indicates that all required files exist.
/// * `Err(io::Error)` if path resolution or retrieval of the current working
///   directory fails.
///
/// # Errors
/// This function may return an `io::Error` if:
/// * The current working directory cannot be determined
/// * Path resolution for error reporting fails
///
/// # Notes
/// * Paths are resolved in three steps:
///   1. Relative paths are resolved against `base_dir`.
///   2. The result is converted into an absolute path for clear error messages.
///   3. The filesystem is queried using `Path::exists`.
/// * No attempt is made to open or validate the contents of the files.
/// * The returned messages are user‑facing and intended for display.
///
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

/// Collects all missing certificate file references across an entire YAML
/// configuration.
///
/// This function iterates over all programs and their associated certificate
/// configurations, checks whether the referenced certificate files exist on disk,
/// and aggregates all missing files into a single list.
///
/// Each missing file is reported with contextual information, including the
/// program ID and certificate name, making the output suitable for user‑friendly
/// error reporting.
///
/// # Parameters
/// * `base_dir` – The base directory used to resolve relative paths defined in the
///   configuration.
/// * `config` – The parsed YAML configuration containing program and certificate
///   definitions.
///
/// # Returns
/// * `Ok(Vec<String>)` containing zero or more error messages describing missing
///   files across all programs.
///   * An empty vector indicates that no certificate files are missing.
/// * `Err(io::Error)` if path resolution or environment‑dependent operations fail.
///
/// # Errors
/// This function may return an `io::Error` if:
/// * Resolving or absolutizing certificate paths fails
/// * Accessing the current working directory fails
///
/// # Notes
/// * Programs without certificates are skipped.
/// * Missing files are reported individually and not deduplicated.
/// * The returned messages are intended for diagnostics or CLI output.
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
