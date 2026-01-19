use crate::config::CloudManagerConfig;
use async_trait::async_trait;
use reqwest::header::AUTHORIZATION;
use reqwest::{Error, Method, Response};
use serde::Serialize;

/// Model for the Cloud Manager client object
#[derive(Debug)]
pub struct CloudManagerClient {
    pub config: CloudManagerConfig,
    pub client: reqwest::Client,
}

/// A generic HTTP interface that leverages `reqwest`.
#[async_trait]
pub trait AdobeConnector {
    async fn perform_request<T>(
        &mut self,
        method: Method,
        path: String,
        body: Option<T>,
        query: Option<Vec<(&str, &str)>>,
    ) -> Result<Response, Error>
    where
        T: Serialize + Send;
}

#[async_trait]
impl AdobeConnector for CloudManagerClient {
    /// Issues HTTP requests with all necessary headers to authenticate with Adobe.
    ///
    /// # Arguments
    ///
    /// * `&mut self`
    /// * `method` - One of reqwest::Method (GET, POST, ...)
    /// * `path` - URL to which the request will be sent
    /// * `body` - Optional body to be sent with the request
    async fn perform_request<T>(
        &mut self,
        method: Method,
        path: String,
        body: Option<T>,
        query: Option<Vec<(&str, &str)>>,
    ) -> Result<Response, Error>
    where
        T: Serialize + Send,
    {
        match method {
            Method::GET => {
                let query_params = match query {
                    None => {
                        vec![("", "")]
                    }
                    Some(q) => q,
                };
                let response = self
                    .client
                    .get(path)
                    .header(AUTHORIZATION, &self.config.access_token)
                    .header("x-gw-ims-org-id", &self.config.organization_id)
                    .header("x-api-key", &self.config.client_id)
                    .query(&query_params)
                    .send()
                    .await?;
                Ok(response)
            }
            Method::PATCH => {
                let request_body = serde_json::to_string(&body.unwrap()).unwrap();
                let response = self
                    .client
                    .patch(path)
                    .header(AUTHORIZATION, &self.config.access_token)
                    .header("x-gw-ims-org-id", &self.config.organization_id)
                    .header("x-api-key", &self.config.client_id)
                    .header("Content-Type", "application/json")
                    .body(request_body)
                    .send()
                    .await?;
                Ok(response)
            }

            Method::PUT => {
                let request_body = serde_json::to_string(&body.unwrap()).unwrap();
                let response = self
                    .client
                    .put(path)
                    .header(AUTHORIZATION, &self.config.access_token)
                    .header("x-gw-ims-org-id", &self.config.organization_id)
                    .header("x-api-key", &self.config.client_id)
                    .header("Content-Type", "application/json")
                    .body(request_body)
                    .send()
                    .await?;
                Ok(response)
            }

            Method::POST => {
                let request_body = serde_json::to_string(&body.unwrap()).unwrap();
                let response = self
                    .client
                    .post(path)
                    .header(AUTHORIZATION, &self.config.access_token)
                    .header("x-gw-ims-org-id", &self.config.organization_id)
                    .header("x-api-key", &self.config.client_id)
                    .header("Content-Type", "application/json")
                    .body(request_body)
                    .send()
                    .await?;
                Ok(response)
            }

            Method::DELETE => {
                let response = self
                    .client
                    .delete(path)
                    .header(AUTHORIZATION, &self.config.access_token)
                    .header("x-gw-ims-org-id", &self.config.organization_id)
                    .header("x-api-key", &self.config.client_id)
                    .header("Content-Type", "application/json")
                    .send()
                    .await?;
                Ok(response)
            }

            _ => panic!("This method is not implemented."),
        }
    }
}

impl From<CloudManagerConfig> for CloudManagerClient {
    fn from(config: CloudManagerConfig) -> Self {
        let http_client = reqwest::Client::new();
        CloudManagerClient {
            config,
            client: http_client,
        }
    }
}
