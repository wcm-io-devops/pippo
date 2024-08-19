use crate::client::{AdobeConnector, CloudManagerClient};
use crate::encryption::decrypt;
use crate::environments::get_environment;
use crate::errors::throw_adobe_api_error;
use crate::models::{Variable, VariableType, VariablesList, VariablesResponse, YamlConfig};
use crate::pipelines::get_pipeline;
use crate::HOST_NAME;
use colored::*;
use reqwest::{Method, StatusCode};
use std::process;
use std::thread::sleep;
use std::time::Duration;

// Make variables comparable - if they have the same name, they are the same.
impl PartialEq for Variable {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// Retrieves environment variables for the specified environment.
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
/// GET https://cloudmanager.adobe.io/api/program/{program_id}/environment/{env_id}/variables
/// ```
pub async fn get_env_vars(
    client: &mut CloudManagerClient,
    program_id: u32,
    env_id: u32,
) -> Result<VariablesList, reqwest::Error> {
    let request_path = format!(
        "{}/api/program/{}/environment/{}/variables",
        HOST_NAME, program_id, env_id
    );
    let response = client
        .perform_request(Method::GET, request_path, None::<()>, None)
        .await?
        .text()
        .await?;
    let variables: VariablesResponse =
        serde_json::from_str(response.as_str()).unwrap_or_else(|_| {
            throw_adobe_api_error(response);
            process::exit(1);
        });
    Ok(variables.variables_list)
}

/// Sets environment variables.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
/// * `env_id` - A u32 that holds the environment ID
/// * `variables` - Reference to an array that holds elements of type VariablesConfig
///
/// # Performed API Request
///
/// ```
/// PATCH https://cloudmanager.adobe.io/api/program/{program_id}/environment/{env_id}/variables
/// ```
pub async fn set_env_vars(
    client: &mut CloudManagerClient,
    program_id: u32,
    env_id: u32,
    variables: &[Variable],
) -> Result<StatusCode, reqwest::Error> {
    let request_path = format!(
        "{}/api/program/{}/environment/{}/variables",
        HOST_NAME, program_id, env_id
    );
    let response = client
        .perform_request(Method::PATCH, request_path, Some(variables), None)
        .await?;
    let response_code = response.status();
    // Print out additional info if request failed
    let response_text = response.text().await?;
    if !response_code.is_success() {
        eprintln!("⚠ PATCH failed: {}", response_text);
    }

    Ok(response_code)
}

/// Sets environment variables that are read from a given YAML file.
///
/// When the target environment is currently updating, the function will retry until its state
/// is back to ready.
///
/// # Arguments
///
/// * `file_path` - String slice that holds the path to the YAML variables config
/// * `client` - A mutable reference to a CloudManagerClient instance
pub async fn set_env_vars_from_file(
    file_path: &str,
    client: &mut CloudManagerClient,
    ci_mode: bool,
) {
    let input = std::fs::read_to_string(file_path).expect("Unable to read file");
    let input: YamlConfig = serde_yaml::from_str(input.as_str()).unwrap_or_else(|err| {
        eprintln!("{} {}", "❌ Malformed YAML: ".red(), err);
        process::exit(1);
    });

    let mut skipped_environment: bool = false;

    let programs = input.programs;

    for p in &programs {
        println!("☁ Program: {}", p.id,);
        for e in p.environments.as_ref().unwrap() {
            let env = get_environment(client, p.id, e.id).await.unwrap();

            println!("{:>4} Environment: {} ({})", "⬛", e.id, env.name);

            // The vector that holds the final variables that will be set or deleted. Will be constructed
            // by comparing the variables that are currently set in Cloud Manager and those in the local
            // YAML config file.
            let mut vars_final: Vec<Variable> = vec![];

            // Check if the targeted environment is ready
            '_retry: loop {
                let env = get_environment(client, p.id, e.id).await.unwrap();

                if env.status == "updating" && ci_mode {
                    skipped_environment = true;
                    eprintln!(
                        "{:>8} Skipped! This environment is currently updating and ci mode (--ci) is active.",
                        "⚠️",
                    );
                    break '_retry;
                } else if env.status == "updating" {
                    eprintln!(
                        "{:>8} This environment is currently updating. Retrying in 1 minute...",
                        "⏲",
                    );
                    sleep(Duration::from_secs(60));
                } else {
                    // To simulate a stateful application of the variables (i.e. remove a variable that is defined
                    // in the cloud, but not in the YAML file), we have to compare them.
                    let vars_yaml = e.variables.clone();

                    // All variables in the YAML are definitely meant to be updated, so they will be
                    // pushed to vars_final.
                    for vy in &vars_yaml {
                        let mut tmp_loop_var = vy.clone();
                        match tmp_loop_var.variable_type {
                            VariableType::String => {
                                // If the value is not secret, just push it to vars_final
                                vars_final.push(tmp_loop_var);
                            }
                            VariableType::SecretString => {
                                // If the value is a secret, check if it's encrypted and decrypt it if that's the case
                                let tmp_loop_var_value = tmp_loop_var.clone().value.unwrap();
                                if tmp_loop_var_value.starts_with("$enc") {
                                    let encrypted_value =
                                        tmp_loop_var_value.split_whitespace().collect::<Vec<_>>();
                                    let decrypted_value = decrypt(encrypted_value[1].to_string());
                                    tmp_loop_var.value = Some(decrypted_value);
                                }
                                vars_final.push(tmp_loop_var);
                            }
                        }
                    }

                    // If a variable is only present on Cloud Manager and not in the YAML, then we
                    // will set its value to None and push it to vars_final, so it will be deleted.
                    let vars_cloud = get_env_vars(client, p.id, e.id).await.unwrap().variables;
                    for vc in vars_cloud {
                        if !vars_yaml.clone().contains(&vc) {
                            let variable_to_be_deleted = Variable {
                                name: vc.name,
                                value: None,
                                variable_type: vc.variable_type,
                                service: vc.service,
                                status: None,
                            };
                            vars_final.push(variable_to_be_deleted);
                        }
                    }

                    for vf in &vars_final {
                        match vf.value {
                            None => {
                                println!("{:>8} DELETING '{}'", "✍", vf.name);
                            }
                            Some(_) => {
                                println!("{:>8} UPDATING '{}'", "✍", vf.name)
                            }
                        }
                    }

                    match set_env_vars(client, p.id, e.id, &vars_final).await {
                        Ok(status) => match status {
                            StatusCode::NO_CONTENT => {
                                println!("{:>8} Success", "✔");
                            }
                            _ => {
                                eprintln!(
                                    "{:>8} {}",
                                    "Error, check output above".red(),
                                    "❌".red()
                                );
                                process::exit(2);
                            }
                        },
                        Err(error) => {
                            eprintln!("{} {}", "❌ API error: ".red().bold(), error);
                            process::exit(1);
                        }
                    }
                    break '_retry;
                }
            }
        }
    }

    if skipped_environment == true {
        eprintln!(
            "\n{} Not all environments were changed because they were updating and --ci mode is active!",
            "⚠️"
        );
        process::exit(2);
    }
}

/// List the user defined variables for an pipeline.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
/// * `pipeline_id` - A u32 that holds the pipeline ID
///
/// # Performed API Request
///
/// ```
/// GET https://cloudmanager.adobe.io/api/program/{programId}/pipeline/{pipelineId}/variables
/// ```
pub async fn get_pipeline_vars(
    client: &mut CloudManagerClient,
    program_id: u32,
    pipeline_id: &u32,
) -> Result<VariablesList, reqwest::Error> {
    let request_path = format!(
        "{}/api/program/{}/pipeline/{}/variables",
        HOST_NAME, program_id, pipeline_id
    );
    let response = client
        .perform_request(Method::GET, request_path, None::<()>, None)
        .await?
        .text()
        .await?;
    let variables: VariablesResponse =
        serde_json::from_str(response.as_str()).unwrap_or_else(|_| {
            throw_adobe_api_error(response);
            process::exit(1);
        });
    Ok(variables.variables_list)
}

/// Sets pipeline variables.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
/// * `program_id` - A u32 that holds the program ID
/// * `pipeline_id` - A u32 that holds the pipeline ID
/// * `variables` - Reference to an array that holds elements of type VariablesConfig
///
/// # Performed API Request
///
/// ```
/// PATCH https://cloudmanager.adobe.io/api/program/{programId}/pipeline/{pipelineId}/variables
/// ```
pub async fn set_pipeline_vars(
    client: &mut CloudManagerClient,
    program_id: u32,
    pipeline_id: u32,
    variables: &[Variable],
) -> Result<StatusCode, reqwest::Error> {
    let request_path = format!(
        "{}/api/program/{}/pipeline/{}/variables",
        HOST_NAME, program_id, pipeline_id
    );
    let response = client
        .perform_request(Method::PATCH, request_path, Some(variables), None)
        .await?;
    let response_code = response.status();
    // Print out additional info if request failed
    let response_text = response.text().await?;
    if !response_code.is_success() {
        eprintln!("⚠ PATCH failed: {}", response_text);
    }

    Ok(response_code)
}

/// Sets pipeline variables that are read from a given YAML file.
///
/// When the target pipeline is currently updating, the function will retry until its state
/// is back to ready.
///
/// # Arguments
///
/// * `file_path` - String slice that holds the path to the YAML variables config
/// * `client` - A mutable reference to a CloudManagerClient instance
pub async fn set_pipeline_vars_from_file(
    file_path: &str,
    client: &mut CloudManagerClient,
    ci_mode: bool,
) {
    let input = std::fs::read_to_string(file_path).expect("Unable to read file");
    let input: YamlConfig = serde_yaml::from_str(input.as_str()).unwrap_or_else(|err| {
        eprintln!("{} {}", "❌ Malformed YAML: ".red(), err);
        process::exit(1);
    });

    let mut skipped_pipeline: bool = false;

    let programs = input.programs;

    for p in &programs {
        println!("☁ Program: {}", p.id,);
        for l in p.pipelines.as_ref().unwrap() {
            let pipeline = get_pipeline(client, p.id, l.id).await.unwrap();

            println!("{:>4} Pipeline: {} ({})", "⬛", l.id, pipeline.name);

            // The vector that holds the final variables that will be set or deleted. Will be constructed
            // by comparing the variables that are currently set in Cloud Manager and those in the local
            // YAML config file.
            let mut vars_final: Vec<Variable> = vec![];

            // Check if the targeted environment is ready
            '_retry: loop {
                let pipeline = get_pipeline(client, p.id, l.id).await.unwrap();

                if pipeline.status == "BUSY" && ci_mode {
                    skipped_pipeline = true;
                    eprintln!(
                        "{:>8} Skipped! This pipeline is currently busy and and ci mode (--ci) is active.",
                        "⚠️",
                    );
                    break '_retry;
                } else if pipeline.status == "BUSY" {
                    eprintln!(
                        "{:>8} This pipeline is currently busy. Retrying in 1 minute...",
                        "⏲",
                    );
                    sleep(Duration::from_secs(60));
                } else {
                    // To simulate a stateful application of the variables (i.e. remove a variable that is defined
                    // in the cloud, but not in the YAML file), we have to compare them.
                    let vars_yaml = l.variables.clone();

                    // All variables in the YAML are definitely meant to be updated, so they will be
                    // pushed to vars_final.
                    for vy in &vars_yaml {
                        let mut tmp_loop_var = vy.clone();
                        match tmp_loop_var.variable_type {
                            VariableType::String => {
                                // If the value is not secret, just push it to vars_final
                                vars_final.push(tmp_loop_var);
                            }
                            VariableType::SecretString => {
                                // If the value is a secret, check if it's encrypted and decrypt it if that's the case
                                let tmp_loop_var_value = tmp_loop_var.clone().value.unwrap();
                                if tmp_loop_var_value.starts_with("$enc") {
                                    let encrypted_value =
                                        tmp_loop_var_value.split_whitespace().collect::<Vec<_>>();
                                    let decrypted_value = decrypt(encrypted_value[1].to_string());
                                    tmp_loop_var.value = Some(decrypted_value);
                                }
                                vars_final.push(tmp_loop_var);
                            }
                        }
                    }

                    // If a variable is only present on Cloud Manager and not in the YAML, then we
                    // will set its value to None and push it to vars_final, so it will be deleted.
                    let vars_cloud = get_pipeline_vars(client, p.id, &l.id)
                        .await
                        .unwrap()
                        .variables;
                    for vc in vars_cloud {
                        if !vars_yaml.clone().contains(&vc) {
                            let variable_to_be_deleted = Variable {
                                name: vc.name,
                                value: None,
                                variable_type: vc.variable_type,
                                service: vc.service,
                                status: None,
                            };
                            vars_final.push(variable_to_be_deleted);
                        }
                    }

                    for vf in &vars_final {
                        match vf.value {
                            None => {
                                println!("{:>8} DELETING '{}'", "✍", vf.name);
                            }
                            Some(_) => {
                                println!("{:>8} UPDATING '{}'", "✍", vf.name)
                            }
                        }
                    }

                    match set_pipeline_vars(client, p.id, l.id, &vars_final).await {
                        Ok(status) => match status {
                            StatusCode::NO_CONTENT => {
                                println!("{:>8} Success", "✔");
                            }
                            _ => {
                                eprintln!(
                                    "{:>8} {}",
                                    "Error, check output above".red(),
                                    "❌".red()
                                );
                                process::exit(2);
                            }
                        },
                        Err(error) => {
                            eprintln!("{} {}", "❌ API error: ".red().bold(), error);
                            process::exit(1);
                        }
                    }
                    break '_retry;
                }
            }
        }
    }

    if skipped_pipeline == true {
        eprintln!(
            "\n{} Not all pipelines were changed because they were busy and --ci mode is active!",
            "⚠️"
        );
        process::exit(2);
    }
}
