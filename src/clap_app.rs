use async_ctrlc::CtrlC;
use chrono::NaiveDate;
use clap::Parser;
use colored::Colorize;
use futures_lite::FutureExt;
use std::process;
use std::str::FromStr;

use crate::auth::obtain_access_token;
use crate::clap_models::*;
use crate::client::CloudManagerClient;
use crate::config::CloudManagerConfig;
use crate::encryption::{decrypt, encrypt};
use crate::logs::{download_log, tail_log};
use crate::models::domain::Domain;
use crate::models::log::{LogType, ServiceType};
use crate::models::variables::{EnvironmentVariableServiceType, PipelineVariableServiceType};

use crate::models::certificates::CertificateList;
use crate::variables::{
    get_env_vars, get_pipeline_vars, set_env_vars_from_file, set_pipeline_vars_from_file,
};
use crate::{certificates, domains, environments, execution, pipelines, programs};

pub async fn init_cli() {
    let cli = Cli::parse();

    // Encryption tooling is somewhat extra to pippo, so we handle this at the very beginning since
    // we don't need a Cloud Manager config for this.
    match &cli.command {
        Some(Commands::Encrypt { input }) => {
            println!("{}", encrypt(input));
            process::exit(0);
        }
        Some(Commands::Decrypt { input }) => {
            println!("{}", decrypt(input.to_string()));
            process::exit(0);
        }
        // All other match cases will be handled later, move on
        _ => {}
    }

    // Read config file
    let cm_config = CloudManagerConfig::from_file(cli.config.as_str());

    // Initialize HTTP client and get access token
    let mut cm_client = CloudManagerClient::from(cm_config);
    obtain_access_token(&mut cm_client).await.unwrap();

    match &cli.command {
        Some(Commands::AccessToken {
            access_token_command,
        }) => match &access_token_command {
            AccessTokenCommands::Print => {
                println!("{}", cm_client.config.access_token);
                process::exit(0);
            }
        },

        Some(Commands::Program { program_command }) => match &program_command {
            ProgramCommands::List => {
                let programs = programs::get_programs(&mut cm_client).await.unwrap();
                println!("{}", serde_json::to_string_pretty(&programs).unwrap());
            }
        },

        Some(Commands::Env { env_command }) => {
            // We do not need any program or environment ID when setting environment variables from an input
            // file, since they are already listed in the file and are handled programmatically.
            #[allow(clippy::collapsible_match)]
            // TODO How can the outer pattern be modified to include the inner pattern?
            if let EnvCommands::Vars { env_vars_command } = &env_command {
                if let EnvVarsCommands::Set { input } = &env_vars_command {
                    println!(
                        "🚀 Patching environment variables from input file {}\n",
                        input
                    );
                    set_env_vars_from_file(input, &mut cm_client, cli.ci_mode, cli.dry_run_mode)
                        .await;
                    process::exit(0);
                }
            }

            // Since all other "env" subcommands need a program ID, we can only run them when it was provided.
            if let Some(program_id) = cli.program {
                match &env_command {
                    EnvCommands::List => {
                        let envs = environments::get_environments(&mut cm_client, program_id)
                            .await
                            .unwrap();
                        println!("{}", serde_json::to_string_pretty(&envs).unwrap());
                    }

                    EnvCommands::Vars { env_vars_command } => {
                        // Since all other "vars" subcommands need an environment ID, we can only run them when it was provided.
                        if let Some(env_id) = cli.env {
                            if let EnvVarsCommands::List = &env_vars_command {
                                let env_vars = get_env_vars(&mut cm_client, program_id, env_id)
                                    .await
                                    .unwrap();
                                println!("{}", serde_json::to_string_pretty(&env_vars).unwrap());
                                if let Some(vf) = env_vars.variables.iter().find(|vf| {
                                    vf.service == EnvironmentVariableServiceType::Invalid
                                }) {
                                    eprintln!(
                                        "{:>8} {}  '{}: {}'",
                                        "⚠".yellow(),
                                        "WARN, invalid service type detected for variable".yellow(),
                                        vf.name,
                                        vf.service
                                    );
                                }
                            }
                        } else {
                            eprintln!("❌ You have to provide a valid Cloud Manager environment ID to run this command!");
                        }
                    }
                }
            } else {
                eprintln!(
                    "❌ You have to provide a valid Cloud Manager program ID to run this command!"
                );
            }
        }

        Some(Commands::Log { log_command }) => {
            // Since all "log" subcommands need program- and environment ID, we can only run them when they were provided.
            if let Some(program_id) = cli.program {
                if let Some(env_id) = cli.env {
                    match log_command {
                        LogCommands::Save { service, log, date } => {
                            let downloaded_file = download_log(
                                &mut cm_client,
                                program_id,
                                env_id,
                                ServiceType::from_str(service).unwrap(),
                                LogType::from_str(log).unwrap(),
                                NaiveDate::from_str(date).unwrap_or_else(|err| {
                                    eprintln!("{}{}", "❌ Cannot parse provided date: ".red(), err);
                                    process::exit(1);
                                }),
                            )
                            .await
                            .unwrap();
                            println!(
                                "{}{}",
                                "Log successfully downloaded and saved at ./".green(),
                                downloaded_file.bold().green()
                            );
                        }

                        LogCommands::Tail { service, log } => {
                            let ctrlc = CtrlC::new().expect("Could not create Ctrl+C handler");
                            ctrlc
                                .race(async {
                                    tail_log(
                                        &mut cm_client,
                                        program_id,
                                        env_id,
                                        ServiceType::from_str(service).unwrap(),
                                        LogType::from_str(log).unwrap(),
                                    )
                                    .await
                                    .unwrap();
                                })
                                .await;
                            println!("{}", "👋 Quitting...".magenta());
                        }
                    }
                }
            }
        }

        Some(Commands::Certificates {
            certificate_command,
        }) => {
            if let CertificateCommands::Manage { input } = &certificate_command {
                if let Err(_e) =
                    certificates::manage_certificates(input.to_string(), &mut cm_client).await
                {
                    process::exit(100);
                }
                process::exit(0);
            } else {
                // Since all "domain" subcommands need a program ID, we can only run them when it was provided.
                if let Some(program_id) = cli.program {
                    match &certificate_command {
                        CertificateCommands::List { start, limit } => {
                            let certificates: CertificateList = certificates::get_certificates(
                                &mut cm_client,
                                program_id,
                                start,
                                limit,
                            )
                            .await
                            .unwrap();

                            println!("{}", serde_json::to_string_pretty(&certificates).unwrap());
                        }
                        CertificateCommands::Manage { input: _ } => {
                            // must be implemented here, but is already run above
                            process::exit(0);
                        }
                    }
                } else {
                    eprintln!(
                        "❌ You have to provide a valid Cloud Manager program ID to run this command!"
                    );
                }
            }
        }

        Some(Commands::Domain { domain_command }) => {
            #[allow(clippy::collapsible_match)]
            if let DomainCommands::Create { input } = &domain_command {
                let _ = domains::create_domains(input.to_string(), &mut cm_client).await;
                println!("🚀 Create Domains succeded. Please Check logs");
                process::exit(0);
            } else {
                // Since all "domain" subcommands need a program ID, we can only run them when it was provided.
                if let Some(program_id) = cli.program {
                    match &domain_command {
                        DomainCommands::List { start, limit } => {
                            let domains =
                                domains::get_domains(&mut cm_client, program_id, start, limit)
                                    .await
                                    .unwrap();
                            if let Some(env_id) = cli.env {
                                let env_i64 = env_id as i64;
                                let filtered_domains: Vec<Domain> = domains
                                    .list
                                    .into_iter()
                                    .filter(|object| object.environment_id.eq(&env_i64))
                                    .collect();
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&filtered_domains).unwrap()
                                );
                            } else {
                                println!("{}", serde_json::to_string_pretty(&domains).unwrap());
                            }
                        }
                        DomainCommands::Create { input: _ } => {
                            // must be implemented here, but is already run above in L163...
                            process::exit(0);
                        }
                    }
                } else {
                    eprintln!("❌ You have to provide a valid Cloud Manager program ID to run this command!");
                }
            }
        }

        Some(Commands::Pipeline { pipeline_command }) => {
            // We do not need any program or pipeline ID when setting pipeline variables from an input
            // file, since they are already listed in the file and are handled programmatically.
            #[allow(clippy::collapsible_match)]
            // TODO How can the outer pattern be modified to include the inner pattern?
            if let PipelineCommands::Vars {
                pipeline_vars_command,
            } = &pipeline_command
            {
                if let PipelineVarsCommands::Set { input } = &pipeline_vars_command {
                    println!("🚀 Patching pipeline variables from input file {}\n", input);
                    set_pipeline_vars_from_file(
                        input,
                        &mut cm_client,
                        cli.ci_mode,
                        cli.dry_run_mode,
                    )
                    .await;
                    process::exit(0);
                }
            }

            // Since all other "pipeline" subcommands need a program ID, we can only run them when it was provided.
            if let Some(program_id) = cli.program {
                match &pipeline_command {
                    PipelineCommands::List => {
                        let pipelines = pipelines::get_pipelines(&mut cm_client, program_id)
                            .await
                            .unwrap();
                        println!("{}", serde_json::to_string_pretty(&pipelines).unwrap());
                    }

                    PipelineCommands::ListExecutions => {
                        if let Some(pipeline_id) = cli.pipeline {
                            let executions =
                                execution::get_executions(&mut cm_client, program_id, pipeline_id)
                                    .await
                                    .unwrap();

                            println!("{}", serde_json::to_string_pretty(&executions).unwrap());
                        } else {
                            eprintln!("❌ You have to provide a valid Cloud Manager pipeline ID to run this command!");
                        }
                    }

                    PipelineCommands::Run => {
                        if let Some(pipeline_id) = cli.pipeline {
                            let execution = pipelines::run_pipeline(
                                &mut cm_client,
                                program_id,
                                pipeline_id,
                                cli.ci_mode,
                            )
                            .await
                            .unwrap();

                            println!(
                                "Execution {:?} started. current Status: {}",
                                execution.id, execution.status
                            );
                        } else {
                            eprintln!("❌ You have to provide a valid Cloud Manager pipeline ID to run this command!");
                        }
                    }
                    PipelineCommands::InvalidateCache => {
                        if let Some(pipeline_id) = cli.pipeline {
                            pipelines::invalidate_pipeline_cache(
                                &mut cm_client,
                                program_id,
                                pipeline_id,
                                cli.ci_mode,
                            )
                            .await;
                        } else {
                            eprintln!("❌ You have to provide a valid Cloud Manager pipeline ID to run this command!");
                        }
                    }

                    PipelineCommands::Vars {
                        pipeline_vars_command,
                    } => {
                        if let Some(pipeline_id) = cli.pipeline {
                            if let PipelineVarsCommands::List = &pipeline_vars_command {
                                let pipeline_vars =
                                    get_pipeline_vars(&mut cm_client, program_id, &pipeline_id)
                                        .await
                                        .unwrap();

                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&pipeline_vars).unwrap()
                                );
                                if let Some(vf) = pipeline_vars
                                    .variables
                                    .iter()
                                    .find(|vf| vf.service == PipelineVariableServiceType::Invalid)
                                {
                                    eprintln!(
                                        "{:>8} {}  '{}: {}'",
                                        "⚠".yellow(),
                                        "WARN, invalid service type detected for variable".yellow(),
                                        vf.name,
                                        vf.service
                                    );
                                }
                            }
                        } else {
                            eprintln!("❌ You have to provide a valid Cloud Manager pipeline ID to run this command!");
                        }
                    }
                }
            } else {
                eprintln!(
                    "❌ You have to provide a valid Cloud Manager program ID to run this command!"
                );
            }
        }

        _ => {}
    }
}
