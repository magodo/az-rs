use std::sync::Arc;

use anyhow::Result;
use azure_core::http::{HttpClient, Url};
use oauth2::{Client, ClientId, ClientSecret, EndpointNotSet, EndpointSet, Scope};
use crate::azidentityext::{flow::OAuthTokenResponse, oauth_http_client::OAuthHttpExecutor};

use super::OAuthClient;

type RefreshTokenClient = OAuthClient<
    EndpointNotSet, // AuthUri is not set
    EndpointNotSet, // DeviceAuthUri is not set
    EndpointNotSet, // IntrospectionUri is not set
    EndpointNotSet, // RevocationUri is not set
    EndpointSet,    // TokenUri is set
>;

pub struct RefreshTokenFlow {
    client: RefreshTokenClient,
}

impl RefreshTokenFlow {
    pub fn new(
        tenant_id: &str,
        client_id: ClientId,
        client_secret: Option<ClientSecret>,
    ) -> Result<Self> {
        let token_url = oauth2::TokenUrl::from_url(
            Url::parse(&format!(
                "https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token"
            ))?,
            // TODO: Wrap in custom error
        );
        let mut client: RefreshTokenClient = Client::new(client_id)
            .set_token_uri(token_url)
            .set_auth_type(oauth2::AuthType::RequestBody);
        if let Some(client_secret) = client_secret {
            client = client.set_client_secret(client_secret);
        }

        Ok(RefreshTokenFlow { client })
    }

    pub async fn exchange(
        self,
        http_client: Arc<dyn HttpClient>,
        refresh_token: &str,
        scopes: &[&str],
    ) -> Result<OAuthTokenResponse> {
        let http_client = |request: oauth2::HttpRequest| {
            let oauth_http_client = OAuthHttpExecutor::new(http_client.clone());
            oauth_http_client.request(request)
        };
        let scopes = scopes.iter().map(ToString::to_string).map(Scope::new);
        let response = self.client.exchange_refresh_token(
            &oauth2::RefreshToken::new(refresh_token.to_string()),
        ).add_scopes(scopes).request_async(
            &http_client
        ).await?;
        Ok(response)
    }
}
