use crate::models::content_requests::ContentRequestsRecord;
use anyhow::Result;
use serde_json::json;

pub async fn ingest_content_requests(
    records: &[ContentRequestsRecord],
    opensearch_url: &str,
    opensearch_index: &str,
    opensearch_username: &str,
    opensearch_password: &str,
    insecure: bool,
) -> Result<()> {
    let mut payload = String::new();

    for record in records {
        payload.push_str(&serde_json::to_string(&json!({
            "index": {
                "_index": opensearch_index,
                "_id": format!("p{}-{}", record.program_id, record.date)
            }
        }))?);
        payload.push('\n');

        payload.push_str(&serde_json::to_string(&json!({
            "@timestamp": record.date,
            "programId": record.program_id,
            "programName": record.program_name,
            "apiCalls": record.api_calls,
            "contentRequests": record.content_requests,
            "pageViews": record.page_views
        }))?);
        payload.push('\n');
    }

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(insecure)
        .build()?;

    let response = client
        .post(format!("{}/_bulk", opensearch_url.trim_end_matches('/')))
        .basic_auth(opensearch_username, Some(opensearch_password))
        .header("Content-Type", "application/x-ndjson")
        .body(payload)
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    println!("OpenSearch status: {}", status);
    println!("{}", body);

    Ok(())
}
