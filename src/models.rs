use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fmt;
use strum_macros::{EnumString, IntoStaticStr};

// Common models used across multiple modules
// -------------------------------------------------------------------------------------------------

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
        skip_serializing_if = "env_var_service_type_is_default"
    )]
    pub service: EnvironmentVariableServiceType,
}

/// Possible types that a variable can have
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VariableType {
    String,
    SecretString,
}

/// Possible service types that an environment variable can have
#[derive(Clone, Debug, Deserialize, Serialize, IntoStaticStr, EnumString, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentVariableServiceType {
    All,
    Author,
    Publish,
    Preview,
}

impl fmt::Display for EnvironmentVariableServiceType {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", format!("{:?}", self).to_lowercase())
    }
}
fn env_var_service_type_is_default(t: &EnvironmentVariableServiceType) -> bool {
    *t == EnvironmentVariableServiceType::All
}

impl EnvironmentVariableServiceType {
    fn default() -> Self {
        EnvironmentVariableServiceType::All
    }
}
/// Possible service types that an environment variable can have
#[derive(Clone, Debug, Deserialize, Serialize, IntoStaticStr, EnumString, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PipelineVariableServiceType {
    Build,
}

impl fmt::Display for crate::models::PipelineVariableServiceType {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", format!("{:?}", self).to_lowercase())
    }
}

impl PipelineVariableServiceType {
    fn default() -> Self {
        PipelineVariableServiceType::Build
    }
}

/// Model for the necessary JWT claims to retrieve an Adobe access token
#[derive(Deserialize, Serialize)]
pub struct JwtClaims {
    pub exp: usize,
    pub iss: String,
    pub sub: String,
    pub aud: String,
    #[serde(rename(serialize = "https://ims-na1.adobelogin.com/s/ent_cloudmgr_sdk"))]
    pub scope_ent_cloudmgr_sdk: bool,
    #[serde(rename(serialize = "https://ims-na1.adobelogin.com/s/ent_aem_cloud_api"))]
    pub scope_ent_aem_cloud_api: bool,
}

/// Helper struct that is used to serialize the access token retrieved from Adobe
#[derive(Debug, Deserialize)]
pub struct BearerResponse {
    pub access_token: String,
}

// Models for representing Cloud Manager programs
// -------------------------------------------------------------------------------------------------

/// Struct that holds the response when requesting /api/programs
#[derive(Deserialize, Serialize)]
pub struct ProgramsResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub programs_list: ProgramsList,
}

/// Model for a list of programs
#[derive(Debug, Deserialize, Serialize)]
pub struct ProgramsList {
    programs: Vec<Program>,
}

/// Model for a program and its relevant metadata
#[derive(Debug, Deserialize, Serialize)]
pub struct Program {
    id: String,
    name: String,
    #[serde(rename(deserialize = "tenantId", serialize = "tenantId"))]
    tenant_id: String,
    enabled: bool,
    status: String,
}

// Models for representing Cloud Manager environments and descendant objects
// -------------------------------------------------------------------------------------------------

/// Struct that holds the response when requesting /api/program/{id}/environments
#[derive(Deserialize, Serialize)]
pub struct EnvironmentsResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub environments_list: EnvironmentsList,
}

/// Model for a list of environments
#[derive(Debug, Deserialize, Serialize)]
pub struct EnvironmentsList {
    environments: Vec<Environment>,
}

/// Model for an environment and its relevant metadata
#[derive(Debug, Deserialize, Serialize)]
pub struct Environment {
    pub name: String,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    env_type: String,
    pub status: String,
    id: String,
    #[serde(rename(deserialize = "programId", serialize = "programId"))]
    program_id: String,
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

// Models for representing Cloud Manager pipelines and descendant objects
// -------------------------------------------------------------------------------------------------

/// Struct that holds the response when requesting /api/programs
#[derive(Deserialize, Serialize)]
pub struct PipelinesResponse {
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub pipelines_list: PipelinesList,
}

/// Model for a list of pipelines
#[derive(Debug, Deserialize, Serialize)]
pub struct PipelinesList {
    pipelines: Vec<Pipeline>,
}

/// Model for a pipeline and its relevant metadata
#[derive(Debug, Deserialize, Serialize)]
pub struct Pipeline {
    pub name: String,
    pub status: String,
    id: String,
    #[serde(rename(deserialize = "programId", serialize = "programId"))]
    program_id: String,
}

// -------------------------------------------------------------------------------------------------

/// Possible types that a service can have
#[derive(Clone, Deserialize, Serialize, IntoStaticStr, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Author,
    Publish,
    Dispatcher,
    #[strum(serialize = "preview_dispatcher")]
    #[serde(rename(deserialize = "preview_dispatcher", serialize = "preview_dispatcher"))]
    PreviewDispatcher,
}

// Models for representing Cloud Manager logs
/// Possible types that a log can have
#[derive(Clone, Deserialize, Serialize, IntoStaticStr, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum LogType {
    AemAccess,
    AemDispatcher,
    AemError,
    AemRequest,
    Cdn,
    HttpdAccess,
    HttpdError,
}

/// Struct that holds the response when requesting /api/program/{id}/environment/{id}/logs
#[derive(Deserialize, Serialize)]
pub struct LogsResponse {
    days: u32,
    name: Vec<String>,
    service: Vec<String>,
    #[serde(rename(deserialize = "_embedded", serialize = "_embedded"))]
    pub embedded: LogsEmbedment,
}

/// Helper struct that is used because of the JSON structure that LogsResponse has
#[derive(Deserialize, Serialize)]
pub struct LogsEmbedment {
    pub downloads: Vec<Log>,
}

/// Struct that represents an available logfile
#[derive(Deserialize, Serialize)]
pub struct Log {
    name: LogType,
    service: ServiceType,
    date: NaiveDate,
}

/// Model for a list of programs
#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutionList {
    #[serde(rename = "executions")]
    list: Vec<Execution>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResponse {
    #[serde(rename = "_embedded")]
    pub execution_list: ExecutionList,
    #[serde(rename = "_totalNumberOfItems")]
    pub total_number_of_items: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    pub id: String,
    pub program_id: String,
    pub pipeline_id: String,
    trigger: String,
    user: String,
    pub status: String,
    created_at: Option<String>,
    updated_at: Option<String>,
    pipeline_type: String,
    pipeline_execution_mode: String,
    finished_at: Option<String>,
}

/// Model for a list of programs
#[derive(Debug, Deserialize, Serialize)]
pub struct DomainList {
    #[serde(rename = "domainNames")]
    pub list: Vec<Domain>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainResponse {
    #[serde(rename = "_embedded")]
    pub domain_list: DomainList,
    #[serde(rename = "_totalNumberOfItems")]
    pub total_number_of_items: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
    pub id: Option<i64>,
    pub name: String,
    pub status: Option<String>,
    pub dns_txt_record: String,
    pub environment_id: i64,
    pub environment_name: Option<String>,
    pub tier: Option<String>,
    pub certificate_id: i64,
    pub certificate_name: Option<String>,
    pub certificate_expire_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinimumDomain {
    pub name: String,
    pub dns_txt_record: String,
    pub environment_id: i64,
    pub certificate_id: i64,
    pub dns_zone: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainResponse {
    #[serde(rename = "type")]
    pub type_field: String,
    pub status: i64,
    pub title: String,
    pub errors: Option<Vec<Error>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub code: String,
    pub message: String,
    pub field: String,
}

// Tail Log

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogTailResponse {
    #[serde(rename = "_embedded")]
    pub embedded: LogTailList,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogTailList {
    pub downloads: Vec<Download>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    #[serde(rename = "_links")]
    pub links: Links,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    #[serde(rename = "http://ns.adobe.com/adobecloud/rel/logs/tail")]
    pub http_ns_adobe_com_adobecloud_rel_logs_tail: Option<HttpNsAdobeComAdobecloudRelLogsTail>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpNsAdobeComAdobecloudRelLogsTail {
    pub href: String,
}
