use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::fmt;
use strum_macros::{EnumString, IntoStaticStr};

/// Model for common cloud manager variables

/// Possible types that a variable can have
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VariableType {
    String,
    SecretString,
}

/// Model for all information about a Cloud Manager environment variable
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnvironmentVariable {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub variable_type: VariableType,
    #[serde(
        default = "EnvironmentVariableServiceType::default",
        skip_serializing_if = "environment_variable_skip_serializing"
    )]
    pub service: EnvironmentVariableServiceType,
}

/// Possible service types that an environment variable can have
#[derive(Clone, Debug, Serialize, IntoStaticStr, EnumString, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentVariableServiceType {
    All,
    Author,
    Publish,
    Preview,
    #[serde(other)]
    Invalid,
}

impl fmt::Display for EnvironmentVariableServiceType {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{}",
            format!("{}", serde_json::to_string(self).unwrap().to_string())
        )
    }
}
fn environment_variable_skip_serializing(t: &EnvironmentVariableServiceType) -> bool {
    *t == EnvironmentVariableServiceType::All
}

impl<'de> serde::Deserialize<'de> for EnvironmentVariableServiceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EnvVarVisitor;

        impl<'de> Visitor<'de> for EnvVarVisitor {
            type Value = EnvironmentVariableServiceType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing an environment variable service type")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    "" => Ok(EnvironmentVariableServiceType::All), // Handle empty string as `All`
                    "author" => Ok(EnvironmentVariableServiceType::Author),
                    "publish" => Ok(EnvironmentVariableServiceType::Publish),
                    "preview" => Ok(EnvironmentVariableServiceType::Preview),
                    _ => Ok(EnvironmentVariableServiceType::Invalid),
                }
            }
        }

        deserializer.deserialize_str(EnvVarVisitor)
    }
}

impl EnvironmentVariableServiceType {
    fn default() -> Self {
        EnvironmentVariableServiceType::All
    }
}

/// Model for all information about a Cloud Manager pipeline variable
/// Model for all information about a Cloud Manager environment variable
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PipelineVariable {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub variable_type: VariableType,
    #[serde(default = "PipelineVariableServiceType::default")]
    pub service: PipelineVariableServiceType,
}

/// Possible service types that an pipeline variable can have
#[derive(Clone, Debug, Deserialize, Serialize, IntoStaticStr, EnumString, PartialEq, Eq)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum PipelineVariableServiceType {
    Build,
    UiTest,
    FunctionalTest,
    #[serde(other)]
    Invalid,
}

impl fmt::Display for PipelineVariableServiceType {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{}",
            format!("{}", serde_json::to_string(self).unwrap().to_string())
        )
    }
}

impl PipelineVariableServiceType {
    fn default() -> Self {
        PipelineVariableServiceType::Build
    }
}

/// Struct to serialize the response of requesting /api/program/{id}/environment/{id}/variables
#[derive(Debug, Deserialize, Serialize)]
pub struct EnvironmentVariablesResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub variables_list: EnvironmentVariablesList,
}

/// Struct to serialize the response of requesting /api/program/{id}/environment/{id}/variables
#[derive(Debug, Deserialize, Serialize)]
pub struct PipelineVariablesResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub variables_list: PipelineVariablesList,
}

/// Struct that holds a list of variables
#[derive(Debug, Deserialize, Serialize)]
pub struct EnvironmentVariablesList {
    pub variables: Vec<EnvironmentVariable>,
}

/// Struct that holds a list of variables
#[derive(Debug, Deserialize, Serialize)]
pub struct PipelineVariablesList {
    pub variables: Vec<PipelineVariable>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tests::read_json_from_file;

    #[test]
    fn deserialize_all_service_environment_variable() {
        let vobj: EnvironmentVariablesResponse =
            read_json_from_file("test/variables/environment_variables_response.json").unwrap();

        let under_test: &EnvironmentVariable = vobj.variables_list.variables.get(0).unwrap();
        assert_eq!(under_test.service, EnvironmentVariableServiceType::All,);
        assert_eq!(under_test.name, "VARIABLE",);
        assert_eq!(
            under_test
                .value
                .clone()
                .unwrap_or("default string".to_string()),
            "no service specified",
        );
        assert_eq!(under_test.variable_type, VariableType::String,);
    }

    #[test]
    fn deserialize_empty_service_environment_variable() {
        let vobj: EnvironmentVariablesResponse =
            read_json_from_file("test/variables/environment_variables_response.json").unwrap();

        let under_test: &EnvironmentVariable = vobj.variables_list.variables.get(1).unwrap();
        assert_eq!(under_test.service, EnvironmentVariableServiceType::All,);
        assert_eq!(under_test.name, "SECRET_VARIABLE",);
        assert_eq!(
            under_test.value.clone().unwrap_or("no_value".to_string()),
            "no_value",
        );
        assert_eq!(under_test.variable_type, VariableType::SecretString,);
    }

    #[test]
    fn deserialize_publish_service_environment_variable() {
        let vobj: EnvironmentVariablesResponse =
            read_json_from_file("test/variables/environment_variables_response.json").unwrap();

        let under_test: &EnvironmentVariable = vobj.variables_list.variables.get(3).unwrap();
        assert_eq!(under_test.service, EnvironmentVariableServiceType::Publish,);
        assert_eq!(under_test.name, "VARIABLE",);
        assert_eq!(
            under_test.value.clone().unwrap_or("no_value".to_string()),
            "publish variable",
        );
        assert_eq!(under_test.variable_type, VariableType::String,);
    }

    #[test]
    fn deserialize_author_service_environment_variable() {
        let vobj: EnvironmentVariablesResponse =
            read_json_from_file("test/variables/environment_variables_response.json").unwrap();

        let under_test: &EnvironmentVariable = vobj.variables_list.variables.get(4).unwrap();
        assert_eq!(under_test.service, EnvironmentVariableServiceType::Author,);
        assert_eq!(under_test.name, "SECRET_VARIABLE",);
        assert_eq!(
            under_test.value.clone().unwrap_or("no_value".to_string()),
            "no_value",
        );
        assert_eq!(under_test.variable_type, VariableType::SecretString,);
    }

    #[test]
    fn deserialize_preview_service_environment_variable() {
        let vobj: EnvironmentVariablesResponse =
            read_json_from_file("test/variables/environment_variables_response.json").unwrap();

        let under_test: &EnvironmentVariable = vobj.variables_list.variables.get(2).unwrap();
        assert_eq!(under_test.service, EnvironmentVariableServiceType::Preview,);
        assert_eq!(under_test.name, "VARIABLE",);
        assert_eq!(
            under_test.value.clone().unwrap_or("no_value".to_string()),
            "preview variable",
        );
        assert_eq!(under_test.variable_type, VariableType::String,);
    }
    #[test]
    fn deserialize_invalid_service_environment_variable() {
        let vobj: EnvironmentVariablesResponse =
            read_json_from_file("test/variables/environment_variables_response.json").unwrap();

        let under_test: &EnvironmentVariable = vobj.variables_list.variables.get(7).unwrap();
        assert_eq!(under_test.service, EnvironmentVariableServiceType::Invalid,);
        assert_eq!(under_test.name, "INVALID_SERVICE_VARIABLE",);
        assert_eq!(
            under_test.value.clone().unwrap_or("no_value".to_string()),
            "invalid service variable",
        );
        assert_eq!(under_test.variable_type, VariableType::String,);
    }

    #[test]
    fn serialize_author_service_environment_variable() {
        let variable: EnvironmentVariable = EnvironmentVariable {
            name: String::from("authorVarName"),
            variable_type: VariableType::String,
            service: EnvironmentVariableServiceType::All,
            value: Some(String::from("authorVarValue"))
        };
        let under_test: String = serde_json::to_string(&variable).unwrap();
        assert_eq!(
            under_test,
            "{\"name\":\"authorVarName\",\"value\":\"authorVarValue\",\"type\":\"string\"}",
        );
    }

    #[test]
    fn serialize_publish_service_environment_variable() {
        let variable: EnvironmentVariable = EnvironmentVariable {
            name: String::from("publishVarName"),
            variable_type: VariableType::SecretString,
            service: EnvironmentVariableServiceType::Publish,
            value: Some(String::from("publishValue"))
        };
        let under_test: String = serde_json::to_string(&variable).unwrap();
        assert_eq!(
            under_test,
            "{\"name\":\"publishVarName\",\"value\":\"publishValue\",\"type\":\"secretString\",\"service\":\"publish\"}",
        );
    }

    #[test]
    fn serialize_preview_service_environment_variable() {
        let variable: EnvironmentVariable = EnvironmentVariable {
            name: String::from("previewVarName"),
            variable_type: VariableType::String,
            service: EnvironmentVariableServiceType::Preview,
            value: Some(String::from("previewValue"))
        };
        let under_test: String = serde_json::to_string(&variable).unwrap();
        assert_eq!(
            under_test,
            "{\"name\":\"previewVarName\",\"value\":\"previewValue\",\"type\":\"string\",\"service\":\"preview\"}",
        );
    }
}
