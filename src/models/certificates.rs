use chrono::{DateTime, Utc, serde::ts_seconds_option};
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

/// Model for a list of certificates
#[derive(Debug, Deserialize, Serialize)]
pub struct CertificateList {
    #[serde(rename = "certificates")]
    pub list: Vec<Certificate>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateResponse {
    #[serde(rename = "_embedded")]
    pub certificate_list: CertificateList,
    #[serde(rename = "_totalNumberOfItems")]
    pub total_number_of_items: i64,
}

/// Possible types that a certificate can have
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
//#[serde(rename_all = "camelCase")]
pub enum CertificateType {
    DV,
    OV,
    EV,
}

/// Possible status that a certificate can have
#[derive(Clone, Debug, PartialEq, EnumString, Deserialize, Serialize)]
pub enum CertificateStatus {
    PENDING,
    VALID,
    EXPIRED,
    #[serde(other)]
    UNKNOWN,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Certificate {
    pub id: Option<i64>,
    //#[serde(skip_serializing)]
    //pub ssl_certificate_type: CertificateType,
    //#[serde(skip_serializing)]
    //pub certificate_status:Option<CertificateStatus>,
    pub serial_number:String,
    pub name: String,
    pub issuer: String,
    #[serde(with = "ts_seconds_option")]
    pub expire_at: Option<DateTime<Utc>>,
    pub common_name: String,
    pub subject_alternative_names: Vec<String>,
    //pub certificate:String,
    //pub chain:String,
    #[serde(with = "ts_seconds_option")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(with = "ts_seconds_option")]
    pub updated_at: Option<DateTime<Utc>>,
}
