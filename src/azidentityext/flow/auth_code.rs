use std::sync::Arc;

use anyhow::Result;
use azure_core::http::{HttpClient, Url};
use oauth2::{Client, HttpRequest, Scope};
use oauth2::{ClientId, ClientSecret};
use oauth2::{EndpointNotSet, EndpointSet};
use super::{OAuthClient, OAuthTokenResponse};

use crate::azidentityext::oauth_http_client::OAuthHttpExecutor;

type AuthorizationCodeClient = OAuthClient<
    EndpointSet,    // AuthUri is set
    EndpointNotSet, // DeviceAuthUri is not set
    EndpointNotSet, // IntrospectionUri is not set
    EndpointNotSet, // RevocationUri is not set
    EndpointSet,    // TokenUri is set
>;

pub struct AuthorizationCodeFlow {
    /// An HTTP client configured for OAuth2 authentication
    pub client: AuthorizationCodeClient,
    /// The authentication HTTP endpoint
    pub authorize_url: Url,
    /// The CSRF token
    pub csrf_state: oauth2::CsrfToken,
    /// The PKCE code verifier
    pub pkce_code_verifier: oauth2::PkceCodeVerifier,
}

impl AuthorizationCodeFlow {
    pub fn new(
        client_id: ClientId,
        client_secret: Option<ClientSecret>,
        tenant_id: &str,
        redirect_url: Url,
        scopes: &[&str],
        prompt: Option<&str>,
        login_hint: Option<&str>,
    ) -> Result<Self> {
        let auth_url = oauth2::AuthUrl::from_url(
            Url::parse(&format!(
                "https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/authorize"
            ))?,
            // TODO: Wrap in custom error
        );
        let token_url = oauth2::TokenUrl::from_url(
            Url::parse(&format!(
                "https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token"
            ))?,
            // TODO: Wrap in custom error
        );

        // Set up the config for the Microsoft Graph OAuth2 process.
        let mut client: AuthorizationCodeClient = Client::new(client_id)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            // Microsoft Graph requires client_id and client_secret in URL rather than
            // using Basic authentication.
            .set_auth_type(oauth2::AuthType::RequestBody)
            .set_redirect_uri(oauth2::RedirectUrl::from_url(redirect_url));
        if let Some(client_secret) = client_secret {
            client = client.set_client_secret(client_secret);
        }

        let scopes = scopes.iter().map(ToString::to_string).map(Scope::new);

        // Microsoft Graph supports Proof Key for Code Exchange (PKCE - https://oauth.net/2/pkce/).
        // Create a PKCE code verifier and SHA-256 encode it as a code challenge.
        let (pkce_code_challenge, pkce_code_verifier) =
            oauth2::PkceCodeChallenge::new_random_sha256();

        // Generate the authorization URL to which we'll redirect the user.
        let mut auth_url_builder = client
            .authorize_url(oauth2::CsrfToken::new_random)
            .add_scopes(scopes)
            .set_pkce_challenge(pkce_code_challenge);
        if let Some(prompt_value) = prompt {
            auth_url_builder = auth_url_builder.add_extra_param("prompt", prompt_value);
        }
        if let Some(login_hint_value) = login_hint {
            auth_url_builder = auth_url_builder.add_extra_param("login_hint", login_hint_value);
        }
        auth_url_builder = auth_url_builder.add_extra_param("response_mode", "form_post");
        // TODO: Enable CAE
        // auth_url_builder = auth_url_builder.add_extra_param("claims", "{\"access_token\":{\"xms_cc\":{\"values\":[\"cp1\"]}}}");

        let (authorize_url, csrf_state) = auth_url_builder.url();

        Ok(AuthorizationCodeFlow {
            client,
            authorize_url,
            csrf_state,
            pkce_code_verifier,
        })
    }

    pub async fn exchange(
        self,
        http_client: Arc<dyn HttpClient>,
        code: oauth2::AuthorizationCode,
    ) -> Result<OAuthTokenResponse> {
        let http_client = |request: HttpRequest| {
            let oauth_http_client = OAuthHttpExecutor::new(http_client.clone());
            oauth_http_client.request(request)
        };

        let token_request = self
            .client
            .exchange_code(code)
            .set_pkce_verifier(self.pkce_code_verifier);

        let token_response = token_request.request_async(&http_client).await?;

        Ok(token_response)
    }
}
