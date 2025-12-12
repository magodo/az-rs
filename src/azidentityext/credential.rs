use std::sync::Arc;

use azure_core::http::HttpClient;

pub mod access_token_credential;
pub mod refreshable_credential;

pub use access_token_credential::AccessTokenCredential;
pub use refreshable_credential::RefreshTokenSession;
pub use refreshable_credential::RefreshableCredential;

use crate::azidentityext::profile::ProfileManager;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait Session {
    type Credential: azure_core::credentials::TokenCredential + Sized;

    async fn get_credential(
        &self,
        http_client: Arc<dyn HttpClient>,
        profile_manager: Option<Arc<dyn ProfileManager>>,
    ) -> anyhow::Result<Self::Credential>;
}
