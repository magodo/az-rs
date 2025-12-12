use async_lock::RwLock;
use azure_core::credentials::{AccessToken, TokenCredential};
use azure_core::http::HttpClient;
use azure_core::time::Duration;
use oauth2::TokenResponse;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::azidentityext::credential::Session;
use crate::azidentityext::flow::refresh_token::RefreshTokenFlow;
use crate::azidentityext::profile::{AuthSession, ProfileManager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenSession {
    access_token: Option<azure_core::credentials::AccessToken>,
    refresh_token: String,
    tenant_id: String,
    client_id: String,
    client_secret: Option<String>,
}

impl RefreshTokenSession {
    pub fn new(
        tenant_id: String,
        client_id: String,
        client_secret: Option<String>,
        refresh_token: String,
        access_token: Option<azure_core::credentials::AccessToken>,
    ) -> Self {
        Self {
            tenant_id,
            client_id,
            client_secret,
            refresh_token,
            access_token,
        }
    }

    pub fn check_expiry(&self, buffer: Duration) -> bool {
        if let Some(token) = &self.access_token {
            token.expires_on <= azure_core::time::OffsetDateTime::now_utc() + buffer
        } else {
            true
        }
    }

    pub async fn refresh(
        &self,
        http_client: Arc<dyn HttpClient>,
        scopes: &[&str],
    ) -> anyhow::Result<Self> {
        let flow = RefreshTokenFlow::new(
            &self.tenant_id,
            oauth2::ClientId::new(self.client_id.clone()),
            self.client_secret
                .as_ref()
                .map(|s| oauth2::ClientSecret::new(s.clone())),
        )?;

        let token_response = flow
            .exchange(http_client.clone(), &self.refresh_token, scopes)
            .await?;

        let access_token = AccessToken {
            token: token_response.access_token().secret().clone().into(),
            expires_on: azure_core::time::OffsetDateTime::now_utc()
                + token_response
                    .expires_in()
                    .expect("OAuth token response should include expires_in"),
        };

        let refresh_token = token_response
            .refresh_token()
            .map(|t| t.secret().clone())
            .unwrap_or_else(|| self.refresh_token.clone());

        Ok(Self {
            tenant_id: self.tenant_id.clone(),
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            refresh_token,
            access_token: Some(access_token),
        })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Session for RefreshTokenSession {
    type Credential = RefreshableCredential;

    async fn get_credential(
        &self,
        http_client: Arc<dyn HttpClient>,
        profile_manager: Option<Arc<dyn ProfileManager>>,
    ) -> anyhow::Result<Self::Credential> {
        Ok(RefreshableCredential::new(
            RwLock::new(self.clone()),
            http_client,
            profile_manager,
        ))
    }
}

#[derive(Debug)]
pub struct RefreshableCredential {
    session: RwLock<RefreshTokenSession>,
    http_client: Arc<dyn HttpClient>,
    profile_manager: Option<Arc<dyn ProfileManager>>,
}

impl RefreshableCredential {
    pub fn new(session: RwLock<RefreshTokenSession>, http_client: Arc<dyn HttpClient>, profile_manager: Option<Arc<dyn ProfileManager>>) -> Self {
        RefreshableCredential {
            session,
            http_client,
            profile_manager,
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl TokenCredential for RefreshableCredential {
    async fn get_token(
        &self,
        scopes: &[&str],
        _: Option<azure_core::credentials::TokenRequestOptions>,
    ) -> azure_core::Result<AccessToken> {
        // Check if the current token is still valid
        let new_session = {
            let session = self.session.read().await;

            if !session.check_expiry(Duration::minutes(5)) {
                tracing::debug!("Access token is still valid, returning existing token");
                return Ok(session
                    .access_token
                    .as_ref()
                    .expect("Access token should be present")
                    .clone());
            }
            tracing::debug!("Access token expired or not present, refreshing using refresh token");
            session
                .refresh(self.http_client.clone(), scopes)
                .await
                .map_err(|e| {
                    azure_core::error::Error::with_message(
                        azure_core::error::ErrorKind::Other,
                        || format!("Failed to refresh token: {}", e),
                    )
                })?
        };

        // Update the session with the new data
        let mut session = self.session.write().await;
        if let Some(profile_manager) = &self.profile_manager {
            profile_manager
                .refresh(&AuthSession::RefreshTokenSession(new_session.clone()))
                .await
                .map_err(|e| {
                    azure_core::error::Error::with_message(
                        azure_core::error::ErrorKind::Other,
                        || format!("Failed to update profile after token refresh: {}", e),
                    )
                })?;
        };
        *session = new_session;

        let access_token = session
            .access_token
            .as_ref()
            .expect("Access token should be present after refresh")
            .clone();

        Ok(access_token)
    }
}
