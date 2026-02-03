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
        // Start building request
        let mut req = self.client.request(method.clone(), &path);

        // Common headers
        req = req
            .header(AUTHORIZATION, &self.config.authorization_header)
            .header("x-gw-ims-org-id", &self.config.organization_id)
            .header("x-api-key", &self.config.client_id);

        // Add query parameters (if any)
        if let Some(params) = query {
            req = req.query(&params);
        }

        // Add JSON body for methods that support it
        match method {
            Method::POST | Method::PUT | Method::PATCH => {
                if let Some(b) = body {
                    req = req.json(&b); // reqwest does serialization + error handling
                } else {
                    req = req.header("Content-Type", "application/json").body("{}");
                    // default empty JSON
                }
            }
            _ => {} // GET and DELETE typically have no bodies
        }

        // Send the request
        let response = req.send().await?;
        Ok(response)
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
