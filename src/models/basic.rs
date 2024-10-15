use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub code: String,
    pub message: String,
    pub field: String,
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
