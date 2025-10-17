use azure_core::http::HttpClient;
use oauth2::{AuthorizationCode, TokenResponse};
use std::sync::Arc;

use crate::azidentityext::flow::auth_code::AuthorizationCodeFlow;
use crate::azidentityext::identity::{AuthSession, Login, Identity};
use crate::azidentityext::credential::refreshable_credential::RefreshableCredential;

mod loopback_server;

pub struct InteractiveBrowserLoginOptions {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_port: u16,
    pub scopes: Vec<String>,
    pub prompt: Option<String>,
    pub login_hint: Option<String>,
    pub success_template: String,
    pub error_template: String,
    pub server_timeout: std::time::Duration,
}

pub struct InteractiveBrowserLogin;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Login for InteractiveBrowserLogin {
    type AuthSession = RefreshTokenSession;
    type LoginOptions = InteractiveBrowserLoginOptions;

    async fn login(&self, http_client: Arc<dyn HttpClient>, login_options: Self::LoginOptions) -> anyhow::Result<Self::AuthSession> {
        let redirect_uri = format!("http://localhost:{}", login_options.redirect_port);
        let auth_code_flow = AuthorizationCodeFlow::new(
            oauth2::ClientId::new(login_options.client_id.clone()),
            login_options.client_secret.as_ref().map(|s| oauth2::ClientSecret::new(s.clone())),
            &login_options.tenant_id,
            azure_core::http::Url::parse(&redirect_uri)?,
            &login_options.scopes.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
            login_options.prompt.as_deref(),
            login_options.login_hint.as_deref(),
        )?;
        let server = loopback_server::LoopbackServer::new(login_options.redirect_port, login_options.success_template, login_options.error_template)?;
        webbrowser::open(&auth_code_flow.authorize_url.to_string())?;
        let code = server.listen_for_code(login_options.server_timeout, auth_code_flow.csrf_state.secret())?;
        let token = auth_code_flow.exchange(http_client, AuthorizationCode::new(code)).await?;
        let refresh_token = token.refresh_token().ok_or_else(|| anyhow::anyhow!("No refresh token received"))?.secret().to_string();
        Ok(RefreshTokenSession {
            refresh_token,
            tenant_id: login_options.tenant_id,
            client_id: login_options.client_id,
            client_secret: login_options.client_secret,
        })
    }
}


pub struct RefreshTokenSession {
    refresh_token: String,
    tenant_id: String,
    client_id: String,
    client_secret: Option<String>,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl AuthSession for RefreshTokenSession {
    async fn logout(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Identity for RefreshTokenSession {
    type Credential = RefreshableCredential;

    async fn get_credential(&self, http_client: Arc<dyn HttpClient>) -> anyhow::Result<Self::Credential> {
        Ok(RefreshableCredential::new(
            self.tenant_id.clone(),
            self.client_id.clone(),
            self.client_secret.clone(),
            self.refresh_token.clone(),
            http_client,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::set_global_logger;
    use tokio;
    use azure_core::credentials::TokenCredential;

    #[tokio::test]
    async fn test_interactive_browser_login() {
        set_global_logger();
        let options = InteractiveBrowserLoginOptions {
            tenant_id: "7b31ddc4-9101-4ef0-a387-79ce181cacdb".to_string(),
            client_id: "04b07795-8ddb-461a-bbee-02f9e1bf7b46".to_string(),
            client_secret: None,
            redirect_port: 47828,
            scopes: vec!["https://management.core.windows.net//.default".to_string(), "offline_access".to_string()],
            prompt: Some("select_account".to_string()),
            login_hint: Some("user@example.com".to_string()),
            success_template: "<html><body><h1>Login Successful</h1></body></html>".to_string(),
            error_template: "<html><body><h1>Login Failed</h1></body></html>".to_string(),
            server_timeout: std::time::Duration::from_secs(300),
        };
        let login = InteractiveBrowserLogin;
        let http_client = azure_core::http::new_http_client();
        let session = login.login(http_client.clone(), options).await.expect("Login failed");
        let credential = session.get_credential(http_client).await.expect("Get credential failed");
        let token = credential.get_token(&["https://management.core.windows.net//.default"], None).await.expect("Get token failed");
        assert!(!token.token.secret().is_empty());
    }
}