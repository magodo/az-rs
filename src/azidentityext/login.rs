pub mod interactive_browser;

use std::sync::Arc;
use azure_core::http::HttpClient;

use anyhow::Result;

use crate::azidentityext::credential::Session;

pub use self::interactive_browser::InteractiveBrowserLogin;
pub use self::interactive_browser::InteractiveBrowserLoginOptions;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait Login {
    type AuthSession: Session;
    type LoginOptions;

    async fn login(&self, http_client: Arc<dyn HttpClient>, login_options: Self::LoginOptions) -> Result<Self::AuthSession>;
}