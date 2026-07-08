use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ContentRequestsResponse {
    pub data: ContentRequestsData,
}

#[derive(Debug, Deserialize)]
pub struct ContentRequestsData {
    pub programs: Vec<ContentRequestsProgram>,
}

#[derive(Debug, Deserialize)]
pub struct ContentRequestsProgram {
    pub id: u32,
    pub name: String,
    pub usage: Vec<ContentRequestsUsage>,
}

#[derive(Debug, Deserialize)]
pub struct ContentRequestsUsage {
    #[serde(rename = "apiCalls")]
    pub api_calls: u64,

    #[serde(rename = "contentRequests")]
    pub content_requests: u64,

    pub date: String,

    #[serde(rename = "pageViews")]
    pub page_views: u64,
}

#[derive(Debug, Serialize)]
pub struct ContentRequestsRecord {
    pub date: String,

    #[serde(rename = "programId")]
    pub program_id: u32,

    #[serde(rename = "programName")]
    pub program_name: String,

    #[serde(rename = "apiCalls")]
    pub api_calls: u64,

    #[serde(rename = "contentRequests")]
    pub content_requests: u64,

    #[serde(rename = "pageViews")]
    pub page_views: u64,
}
