use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use strum_macros::{EnumString, IntoStaticStr};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tests::read_json_from_file;

    #[test]
    fn deserialize_logs_response() {
        let vobj: LogsResponse = read_json_from_file("test/test_log_response.json").unwrap();

        assert_eq!(vobj.embedded.downloads.len(), 3);
    }
}
