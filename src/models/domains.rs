use serde::{Deserialize, Serialize};

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
