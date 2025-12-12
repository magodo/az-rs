use azure_core::credentials::TokenRequestOptions;
use azure_core::credentials::{AccessToken, TokenCredential};
use azure_core::time::{Duration, OffsetDateTime};
use azure_core::Result;

/// Authenticates an application with an existing access token.
#[derive(Debug)]
pub struct AccessTokenCredential {
    token: AccessToken,
}

impl AccessTokenCredential {
    pub fn new(token: String) -> Self {
        Self {
            token: AccessToken {
                token: token.into(),
                expires_on: OffsetDateTime::now_utc() + Duration::hours(1),
            },
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl TokenCredential for AccessTokenCredential {
    async fn get_token(&self, _: &[&str], _: Option<TokenRequestOptions>) -> Result<AccessToken> {
        Ok(self.token.clone())
    }
}
