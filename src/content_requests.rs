use crate::client::{AdobeConnector, CloudManagerClient};
use crate::models::content_requests::{ContentRequestsRecord, ContentRequestsResponse};
use crate::ASSETS_REPORTING_HOST_NAME;
use chrono::{Duration, NaiveDate};
use reqwest::Method;

const DAILY_SLICE_DAYS: i64 = 60;
const MONTHLY_SLICE_DAYS: i64 = 360;

pub async fn download_content_requests(
    client: &mut CloudManagerClient,
    start_date: &str,
    end_date: &str,
    time_unit: &str,
    program_id_filter: Option<&str>,
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
            program_id_filter,
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
    program_id_filter: Option<&str>,
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

    Ok(flatten_content_requests(parsed, program_id_filter))
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
    program_id_filter: Option<&str>,
) -> Vec<ContentRequestsRecord> {
    response
        .data
        .programs
        .into_iter()
        .filter(|program| program_id_filter.is_none_or(|id| program.id.to_string() == id))
        .flat_map(|program| {
            let program_id = program.id;
            let program_name = program.name;

            program
                .usage
                .into_iter()
                .map(move |usage| ContentRequestsRecord {
                    date: format!("{}T00:00:00Z", usage.date),
                    program_id,
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
            ("2025-01-01".to_string(), "2025-03-01".to_string())
        );
        assert_eq!(
            slices[1],
            ("2025-03-02".to_string(), "2025-04-15".to_string())
        );
    }

    #[test]
    fn builds_monthly_slices() {
        let slices = build_date_slices("2025-01-01", "2026-03-31", MONTHLY_SLICE_DAYS);

        assert_eq!(slices.len(), 2);
        assert_eq!(
            slices[0],
            ("2025-01-01".to_string(), "2025-12-26".to_string())
        );
        assert_eq!(
            slices[1],
            ("2025-12-27".to_string(), "2026-03-31".to_string())
        );
    }
}
