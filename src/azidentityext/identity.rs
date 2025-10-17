pub mod interactive_browser;

use std::sync::Arc;
use azure_core::http::HttpClient;

use anyhow::Result;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait Login {
    type AuthSession: AuthSession;
    type LoginOptions;

    async fn login(&self, http_client: Arc<dyn HttpClient>, login_options: Self::LoginOptions) -> Result<Self::AuthSession>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait Identity {
    type Credential: azure_core::credentials::TokenCredential + Sized;

    async fn get_credential(&self, http_client: Arc<dyn HttpClient>) -> Result<Self::Credential>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait AuthSession: Identity {
    async fn logout(&self) -> Result<()>;
}