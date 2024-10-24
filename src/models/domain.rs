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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::BufReader, path::Path};

    fn read_user_from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<DomainResponse, Box<dyn std::error::Error>> {
        // Open the file in read-only mode with buffer.
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Read the JSON contents of the file as an instance of `User`.
        let u = serde_json::from_reader(reader)?;

        // Return the `User`.
        Ok(u)
    }

    #[test]
    fn dedeserialize_domain_response() {
        // Read the JSON contents of the file as an instance of `User`.
        let vobj: DomainResponse = read_user_from_file("test/test_domain_response.json").unwrap();
        assert_eq!(vobj.domain_list.list.len(), 20);
    }
}
