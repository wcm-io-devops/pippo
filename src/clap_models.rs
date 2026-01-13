use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser)]
#[clap(
author,
version,
about,
long_about = None
)]
#[clap(propagate_version = true)]
pub struct Cli {
    /// Path to JSON config file
    #[clap(short, long, value_parser, default_value_t = String::from("./pippo.json"), env = "PIPPO_CONFIG")]
    pub config: String,

    /// Cloud Manager program ID
    #[clap(short, long, value_parser, global = true, env = "PIPPO_PROGRAM_ID")]
    pub program: Option<u32>,

    /// Cloud Manager environment ID
    #[clap(short, long, value_parser, global = true, env = "PIPPO_ENVIRONMENT_ID")]
    pub env: Option<u32>,

    /// Pipeline ID
    #[clap(
        short = 'i',
        long,
        value_parser,
        global = true,
        env = "PIPPO_PIPELINE_ID"
    )]
    pub pipeline: Option<u32>,

    /// skips resources that can not be updated at the moment (e.g. running pipelines)
    #[clap(long = "ci", global = true, action = ArgAction::SetTrue )]
    pub ci_mode: bool,

    /// Only log but to not apply any changes
    #[clap(long = "dry-run", global = true, action = ArgAction::SetTrue )]
    pub dry_run_mode: bool,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Encrypt a string using the provided .cryptkey file
    Encrypt {
        /// The string you want to encrypt
        #[clap(value_parser)]
        input: String,
    },

    /// Decrypt a string using the provided .cryptkey file
    Decrypt {
        /// The string you want to decrypt
        #[clap(value_parser)]
        input: String,
    },

    /// Adobe I/O access_token utilities
    AccessToken {
        #[clap(subcommand)]
        access_token_command: AccessTokenCommands,
    },

    /// Tools to interact with Cloud Manager programs
    Program {
        #[clap(subcommand)]
        program_command: ProgramCommands,
    },

    /// Tools to interact with Cloud Manager environments
    Env {
        #[clap(subcommand)]
        env_command: EnvCommands,
    },

    /// Tools to access logs of the specified Cloud Manager environment
    Log {
        #[clap(subcommand)]
        log_command: LogCommands,
    },

    /// Tools to interact with Cloud Manager pipelines
    Pipeline {
        #[clap(subcommand)]
        pipeline_command: PipelineCommands,
    },

    /// Tools to interact with Cloud Manager domains
    Domain {
        #[clap(subcommand)]
        domain_command: DomainCommands,
    },

    /// Tools to interact with Cloud Manager certificates
    Certificates {
        #[clap(subcommand)]
        certificate_command: CertificateCommands,
    },
}

#[derive(Subcommand)]
pub enum AccessTokenCommands {
    /// prints access_token to stdout
    Print,
}

#[derive(Subcommand)]
pub enum ProgramCommands {
    /// List all programs
    List,
}

#[derive(Subcommand)]
pub enum EnvCommands {
    /// List all environments of the specified program
    List,

    /// Read or update Cloud Manager environment variables
    Vars {
        #[clap(subcommand)]
        env_vars_command: EnvVarsCommands,
    },
}

#[derive(Subcommand)]
pub enum EnvVarsCommands {
    /// List all environment variables
    List,
    /// Update environment variables read from YAML file
    Set {
        /// Path to input file
        #[clap(value_parser, value_name = "FILE")]
        input: String,
    },
}

#[derive(Subcommand)]
pub enum LogCommands {
    /// Download the specified logfile
    Save {
        /// Name of service
        #[clap(short, long, value_parser, possible_values = vec!["author", "publish", "dispatcher", "preview_dispatcher"])]
        service: String,

        /// Name of log file
        #[clap(short, long, value_parser, possible_values = vec!["aemaccess", "aemdispatcher", "aemerror", "aemrequest", "cdn", "httpdaccess", "httpderror"])]
        log: String,

        /// Date of which specified log file will be downloaded
        #[clap(short, long, value_parser, value_name = "YYYY-MM-DD")]
        date: String,
    },

    /// Tail the latest of the specified logfile
    Tail {
        /// Name of service
        #[clap(short, long, value_parser, possible_values = vec!["author", "publish", "dispatcher", "preview_dispatcher"])]
        service: String,

        /// Name of log file
        #[clap(short, long, value_parser, possible_values = vec!["aemaccess", "aemdispatcher", "aemerror", "aemrequest", "cdn", "httpdaccess", "httpderror"])]
        log: String,
    },
}

#[derive(Subcommand)]
pub enum PipelineCommands {
    /// List all pipelines of the specified program
    List,
    /// Runs a pipeline
    Run,
    /// Prints all executions
    ListExecutions,
    /// Read or update Cloud Manager environment variables
    Vars {
        #[clap(subcommand)]
        pipeline_vars_command: PipelineVarsCommands,
    },
    /// Invalidate pipeline cache,
    InvalidateCache,
}

#[derive(Subcommand)]
pub enum PipelineVarsCommands {
    /// List all pipeline variables
    List,
    /// Update pipeline variables read from YAML file
    Set {
        /// Path to input file
        #[clap(value_parser, value_name = "FILE")]
        input: String,
    },
}

#[derive(Subcommand)]
pub enum DomainCommands {
    /// List all domains of the specified program
    List {
        /// Pagination start parameter
        #[clap(short, long, value_parser, default_value_t = 0)]
        start: u32,
        /// Pagination limit parameter
        #[clap(short, long, value_parser, default_value_t = 1000)]
        limit: u32,
    },
    /// Creates domains based upon a provided file
    Create {
        #[clap(value_parser, value_name = "FILE")]
        input: String,
    },
}

#[derive(Subcommand)]
pub enum CertificateCommands {
    /// List all certificates of the specified program
    List {
        /// Pagination start parameter
        #[clap(short, long, value_parser, default_value_t = 0)]
        start: u32,
        /// Pagination limit parameter
        #[clap(short, long, value_parser, default_value_t = 1000)]
        limit: u32,
    },
    /// Creates/Updates certificates
    Manage {
        #[clap(value_parser, value_name = "FILE")]
        input: String,
    },
}