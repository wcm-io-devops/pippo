use crate::client::CloudManagerClient;
use crate::config::{AuthStrategy, Scope};
use crate::models::auth::{BearerResponse, JwtClaims};
use crate::IMS_ENDPOINT;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use log::debug;

/// Generates a JWT to authenticate with the Adobe API.
///
/// # Arguments
///
/// * `client` - A reference to a CloudManagerClient instance
fn generate_jwt(client: &CloudManagerClient) -> String {
    let date = Utc::now() + Duration::minutes(1);
    debug!("scope from config: {:?}", client.config.scope);
    let claims = JwtClaims {
        exp: date.timestamp() as usize,
        iss: client.config.organization_id.clone(),
        sub: client.config.technical_account_id.clone(),
        aud: format!(
            "https://{}/c/{}",
            IMS_ENDPOINT,
            client.config.client_id.clone()
        ),
        scope_ent_cloudmgr_sdk: client.config.scope == Scope::EntCloudmgrSdk,
        scope_ent_aem_cloud_api: client.config.scope == Scope::EntAemCloudApi,
    };

    let private_key = client.config.private_key.as_bytes();
    encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(private_key).expect("Private key is in the wrong format"),
    )
    .unwrap()
}

/// Uses a JWT to obtain an access token from Adobe.
///
/// # Arguments
///
/// * `client` - A mutable reference to a CloudManagerClient instance
///
/// # Performed API Request
///
/// ```
/// POST https://ims-na1.adobelogin.com/ims/exchange/jwt/
/// ```
pub async fn obtain_access_token(client: &mut CloudManagerClient) -> Result<(), reqwest::Error> {
    if client.config.auth_strategy == AuthStrategy::JWT {
        obtain_jwt_token(client).await?;
    } else {
        obtain_oauth_token(client).await?;
    }
    Ok(())
}

async fn obtain_oauth_token(client: &mut CloudManagerClient) -> Result<(), reqwest::Error> {
    //client.config.jwt = generate_jwt(client);
    let form_params = [
        ("client_id", client.config.client_id.clone()),
        ("client_secret", client.config.client_secret.clone()),
        ("scope", "read_pc.dma_aem_ams,openid,AdobeID,read_organizations,additional_info.projectedProductContext".to_owned()),
        ("grant_type", "client_credentials".to_owned()),
    ];

    let token = &client
        .client
        .post(format!("https://{}/ims/token/v3/", IMS_ENDPOINT))
        .form(&form_params)
        .send()
        .await?
        .text()
        .await?;

    let bearer_response: BearerResponse = serde_json::from_str(token)
        .unwrap_or_else(|_| panic!("Unable to authenticate: {}", token.as_str()));
    client.config.access_token = format!("Bearer {}", bearer_response.access_token);
    Ok(())
}

async fn obtain_jwt_token(client: &mut CloudManagerClient) -> Result<(), reqwest::Error> {
    client.config.jwt = generate_jwt(client);
    let form_params = [
        ("client_id", client.config.client_id.clone()),
        ("client_secret", client.config.client_secret.clone()),
        ("jwt_token", client.config.jwt.clone()),
    ];

    let token = &client
        .client
        .post(format!("https://{}/ims/exchange/jwt/", IMS_ENDPOINT))
        .form(&form_params)
        .send()
        .await?
        .text()
        .await?;

    let bearer_response: BearerResponse = serde_json::from_str(token)
        .unwrap_or_else(|_| panic!("Unable to authenticate: {}", token.as_str()));
    client.config.access_token = bearer_response.access_token;
    Ok(())
}
