use serde::{Deserialize, Serialize};
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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::tests::read_json_from_file;

    #[test]
    fn serialize_domain_list() {
        // Read the JSON contents of the file as an instance of `User`.
        let vobj: BearerResponse =
            read_json_from_file("test/test_auth_bearer_response.json").unwrap();
        assert_eq!(vobj.access_token, "das.ist.ein.token");
    }
    #[test]
    fn serialize_jwt_claims() {
        let vobj: JwtClaims = read_json_from_file("test/test_auth_jwt_response.json").unwrap();

        assert_eq!(
            vobj.aud,
            "https://ims-na1.adobelogin.com/c/4df5gh....."
        );
        assert_eq!(vobj.exp, 1550001438);
        assert_eq!(vobj.iss, "C74F69D7594880280.....@AdobeOrg");
        assert_eq!(vobj.sub, "6657031C5C095BB40A4.....@techacct.adobe.com");
        assert_eq!(vobj.scope_ent_aem_cloud_api, false);
        assert_eq!(vobj.scope_ent_cloudmgr_sdk, true);
    }
}
