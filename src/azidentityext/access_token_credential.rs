use azure_core::credentials::TokenRequestOptions;
use azure_core::credentials::{AccessToken, TokenCredential};
use azure_core::Result;
use std::{str, sync::Arc};

/// Authenticates an application with an existing access token.
#[derive(Debug)]
pub struct AccessTokenCredential {
    token: AccessToken,
}

impl AccessTokenCredential {
    pub fn new(token: String) -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            token: serde_json::from_str(&token)?,
        }))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl TokenCredential for AccessTokenCredential {
    async fn get_token(&self, _: &[&str], _: Option<TokenRequestOptions>) -> Result<AccessToken> {
        Ok(self.token.clone())
    }
}
