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
