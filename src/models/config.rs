use serde::{Deserialize, Serialize};

use super::variables::{EnvironmentVariable, PipelineVariable};

/// Model for all programs that will be read from the configuration YAML
#[derive(Debug, Deserialize, Serialize)]
pub struct YamlConfig {
    pub programs: Vec<ProgramsConfig>,
}

/// Model for a program's ID and all its environments that will be read from the configuration YAML
#[derive(Debug, Deserialize, Serialize)]
pub struct ProgramsConfig {
    pub id: u32,
    pub environments: Option<Vec<EnvironmentsConfig>>,
    pub pipelines: Option<Vec<PipelinesConfig>>,
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
    fn serialize_domain_config() {
        let vobj: YamlConfig = read_yaml_from_file("test/test_yaml_config.yml").unwrap();

        assert_eq!(vobj.programs.len(), 1);
        assert_eq!(vobj.programs.first().unwrap().id, 222222);
        assert_eq!(vobj.programs.first().unwrap().pipelines.is_some(), true);
    }
}
