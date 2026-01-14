use super::variables::{EnvironmentVariable, PipelineVariable};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{fs, process};

/// Model for all programs that will be read from the configuration YAML
#[derive(Debug, Deserialize, Serialize)]
pub struct YamlConfig {
    pub programs: Vec<ProgramsConfig>,
}

impl YamlConfig {
    pub fn from_file(path: String) -> Self {
        let data = fs::read_to_string(&path)
            .unwrap_or_else(|_| {
                eprintln!("[ERROR] Unable to find/load yaml config at path '{}'. The documentation is available at https://github.com/wcm-io-devops/pippo", &path);
                std::process::exit(1)
            });
        let input: YamlConfig = serde_yaml::from_str(data.as_str()).unwrap_or_else(|err| {
            eprintln!("{} {}", "❌ Malformed YAML: ".red(), err);
            process::exit(1);
        });
        input
    }
}

/// Model for a program's ID and all its environments that will be read from the configuration YAML
#[derive(Debug, Deserialize, Serialize)]
pub struct ProgramsConfig {
    pub id: u32,
    pub environments: Option<Vec<EnvironmentsConfig>>,
    pub pipelines: Option<Vec<PipelinesConfig>>,
    pub certificates: Option<Vec<CertificateConfig>>,
}

/// Model for an environment's ID and all its variables that will be read from the configuration YAML
#[derive(Debug, Deserialize, Serialize)]
pub struct CertificateConfig {
    pub name: String,
    pub id: Option<i64>,
    pub certificate: String,
    pub chain: String,
    pub key: String,
}

/// Model for an environment's ID and all its variables that will be read from the configuration YAML
#[derive(Debug, Deserialize, Serialize)]
pub struct DomainConfig {
    pub domainname: String,
    pub certificate_id: i64,
}

/// Model for an environment's ID and all its variables that will be read from the configuration YAML
#[derive(Debug, Deserialize, Serialize)]
pub struct EnvironmentsConfig {
    pub id: u32,
    pub variables: Vec<EnvironmentVariable>,
    pub domains: Option<Vec<DomainConfig>>,
}

/// Model for a pipeline's ID and all its variables that will be read from the configuration YAML
#[derive(Debug, Deserialize, Serialize)]
pub struct PipelinesConfig {
    pub id: u32,
    pub variables: Vec<PipelineVariable>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tests::read_yaml_from_file;

    #[test]
    fn deserialize_yaml_config() {
        let vobj: YamlConfig = read_yaml_from_file("test/test_yaml_config.yml").unwrap();

        assert_eq!(vobj.programs.len(), 1);
        assert_eq!(vobj.programs.first().unwrap().id, 222222);
        assert_eq!(vobj.programs.first().unwrap().pipelines.is_some(), true);
    }
}
