//! `pippo` is a fast CLI to communicate with Adobe's Cloud Manager. It mainly uses `clap` and
//! `reqwest` under the hood.
//!
//! The main logic for the application can be found in `clap_app.rs`.
//!
//! Have a look at the
//! [Cloud Manager API specification](https://www.adobe.io/experience-cloud/cloud-manager/reference/api/)
//! if you have questions about the various API endpoints.

extern crate core;

mod auth;
mod certificates;
mod clap_app;
mod clap_models;
mod client;
mod config;
mod domains;
mod encryption;
mod environments;
mod errors;
mod execution;
mod logs;
mod models;
mod pipelines;
mod programs;
mod variables;

use crate::clap_app::init_cli;

const HOST_NAME: &str = "https://cloudmanager.adobe.io";
const IMS_ENDPOINT: &str = "ims-na1.adobelogin.com";

#[tokio::main]
async fn main() {
    // enable logger
    env_logger::init();

    // Enable virtual terminal to correctly colorize output on Windows 10 machines
    #[cfg(target_os = "windows")]
    colored::control::set_virtual_terminal(true).unwrap();

    init_cli().await;
}
