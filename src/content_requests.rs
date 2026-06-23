use crate::client::{AdobeConnector, CloudManagerClient};
use crate::ASSETS_REPORTING_HOST_NAME;
use chrono::{Duration, NaiveDate};
use reqwest::Method;
use serde::{Deserialize, Serialize};

const DAILY_SLICE_DAYS: i64 = 90;
const MONTHLY_SLICE_DAYS: i64 = 365;

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

    #[serde(rename = "programName")]
    pub program_name: String,

    #[serde(rename = "apiCalls")]
    pub api_calls: u64,

    #[serde(rename = "contentRequests")]
    pub content_requests: u64,

    #[serde(rename = "pageViews")]
    pub page_views: u64,
}

pub async fn download_content_requests(
    client: &mut CloudManagerClient,
    start_date: &str,
    end_date: &str,
    time_unit: &str,
    program_name_filter: Option<&str>,
) -> Result<Vec<ContentRequestsRecord>, reqwest::Error> {
    let slice_days = if time_unit == "daily" {
        DAILY_SLICE_DAYS
    } else {
        MONTHLY_SLICE_DAYS
    };

    let ranges = build_date_slices(start_date, end_date, slice_days);
    let mut records: Vec<ContentRequestsRecord> = Vec::new();

    for (slice_start, slice_end) in ranges {
        let mut slice_records = download_content_requests_slice(
            client,
            &slice_start,
            &slice_end,
            time_unit,
            program_name_filter,
        )
        .await?;

        records.append(&mut slice_records);
    }

    Ok(records)
}

async fn download_content_requests_slice(
    client: &mut CloudManagerClient,
    start_date: &str,
    end_date: &str,
    time_unit: &str,
    program_name_filter: Option<&str>,
) -> Result<Vec<ContentRequestsRecord>, reqwest::Error> {
    let path = format!(
        "{}/statistics/contentRequestsUsage",
        ASSETS_REPORTING_HOST_NAME
    );

    let query = vec![
        ("startDate", start_date),
        ("endDate", end_date),
        ("timeUnit", time_unit),
    ];

    let response = client
        .perform_assets_reporting_request::<()>(Method::GET, path, None, Some(query))
        .await?;

    let parsed = response.json::<ContentRequestsResponse>().await?;

    Ok(flatten_content_requests(parsed, program_name_filter))
}

fn build_date_slices(start_date: &str, end_date: &str, slice_days: i64) -> Vec<(String, String)> {
    let start = NaiveDate::parse_from_str(start_date, "%Y-%m-%d").unwrap();
    let end = NaiveDate::parse_from_str(end_date, "%Y-%m-%d").unwrap();

    let mut ranges = Vec::new();
    let mut current_start = start;

    while current_start <= end {
        let current_end = std::cmp::min(current_start + Duration::days(slice_days - 1), end);

        ranges.push((
            current_start.format("%Y-%m-%d").to_string(),
            current_end.format("%Y-%m-%d").to_string(),
        ));

        current_start = current_end + Duration::days(1);
    }

    ranges
}

fn flatten_content_requests(
    response: ContentRequestsResponse,
    program_name_filter: Option<&str>,
) -> Vec<ContentRequestsRecord> {
    response
        .data
        .programs
        .into_iter()
        .filter(|program| program_name_filter.is_none_or(|name| program.name == name))
        .flat_map(|program| {
            let program_name = program.name;

            program
                .usage
                .into_iter()
                .map(move |usage| ContentRequestsRecord {
                    date: format!("{}T00:00:00Z", usage.date),
                    program_name: program_name.clone(),
                    api_calls: usage.api_calls,
                    content_requests: usage.content_requests,
                    page_views: usage.page_views,
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_daily_slices() {
        let slices = build_date_slices("2025-01-01", "2025-04-15", DAILY_SLICE_DAYS);

        assert_eq!(slices.len(), 2);
        assert_eq!(
            slices[0],
            ("2025-01-01".to_string(), "2025-03-31".to_string())
        );
        assert_eq!(
            slices[1],
            ("2025-04-01".to_string(), "2025-04-15".to_string())
        );
    }

    #[test]
    fn builds_monthly_slices() {
        let slices = build_date_slices("2025-01-01", "2026-03-31", MONTHLY_SLICE_DAYS);

        assert_eq!(slices.len(), 2);
        assert_eq!(
            slices[0],
            ("2025-01-01".to_string(), "2025-12-31".to_string())
        );
        assert_eq!(
            slices[1],
            ("2026-01-01".to_string(), "2026-03-31".to_string())
        );
    }
}
