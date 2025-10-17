use azure_core::credentials::{AccessToken, TokenCredential};
use azure_core::http::HttpClient;
use oauth2::TokenResponse;
use std::sync::{Arc, RwLock};

use crate::azidentityext::flow::refresh_token::RefreshTokenFlow;

#[derive(Debug)]
pub struct RefreshableCredential {
    access_token: RwLock<Option<AccessToken>>,
    refresh_token: RwLock<Option<String>>,
    tenant_id: String,
    client_id: String,
    client_secret: Option<String>,
    http_client: Arc<dyn HttpClient>,
}

impl RefreshableCredential {
    pub fn new(tenant_id: String, client_id: String, client_secret: Option<String>, refresh_token: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            tenant_id,
            client_id,
            client_secret,
            refresh_token: RwLock::new(Some(refresh_token)),
            http_client,
            access_token: RwLock::new(None),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl TokenCredential for RefreshableCredential {
    async fn get_token(&self, scopes: &[&str], _: Option<azure_core::credentials::TokenRequestOptions>) -> azure_core::Result<AccessToken> {
        // Here you would implement the logic to refresh the access token using the refresh token.
        // This is a placeholder implementation.
        if let Ok(token) = &self.access_token.read() {
            if let Some(token) = token.as_ref() {
                if token.expires_on > azure_core::time::OffsetDateTime::now_utc() + azure_core::time::Duration::minutes(5) {
                    return Ok(token.clone());
                }
            }
        }

        let flow = RefreshTokenFlow::new(
            &self.tenant_id,
            oauth2::ClientId::new(self.client_id.clone()),
            self.client_secret.as_ref().map(|s| oauth2::ClientSecret::new(s.clone())),
        ).map_err(|e| azure_core::error::Error::with_message(azure_core::error::ErrorKind::Other, || format!("Failed to create refresh token flow: {}", e)))?;

        let refresh_token = self.refresh_token.read().map_err(|e| azure_core::error::Error::with_message(azure_core::error::ErrorKind::Other, || format!("Failed to acquire read lock for refresh token: {}", e)))?.clone();

        if let Some(refresh_token) = refresh_token {
            flow.exchange(self.http_client.clone(), &refresh_token, scopes).await.map_err(|e| azure_core::error::Error::with_message(azure_core::error::ErrorKind::Other, || format!("Failed to exchange refresh token: {}", e))).map(|token_response| {
                let access_token = AccessToken {
                    token: token_response.access_token().secret().clone().into(),
                    expires_on: azure_core::time::OffsetDateTime::now_utc() + token_response.expires_in().unwrap(),
                };
                if let Ok(mut write_lock) = self.access_token.write() {
                    *write_lock = Some(access_token.clone());
                }
                let new_refresh_token = token_response.refresh_token().map(|rt| rt.secret().clone());
                if let Ok(mut write_lock) = self.refresh_token.write() {
                    *write_lock = new_refresh_token;
                }
                access_token
            })
        } else {
            Err(azure_core::error::Error::with_message(azure_core::error::ErrorKind::Other, || "No refresh token available".to_string()))
        }
        
    }
}
