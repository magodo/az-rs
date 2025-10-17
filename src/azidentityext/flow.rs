pub mod auth_code;
pub mod refresh_token;

use oauth2::ExtraTokenFields;
use oauth2::{EndpointNotSet};
use serde::{Deserialize, Serialize};

/// Custom extra token fields implementation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CustomTokenFields {
    /// The ID token returned by the OAuth2 server.
    #[serde(default, rename = "id_token", skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
}

/// The ExtraTokenFields trait is just a marker trait with bounds
/// DeserializeOwned + Debug + Serialize, no methods to implement
impl ExtraTokenFields for CustomTokenFields {}

type OAuthTokenResponse =
    oauth2::StandardTokenResponse<CustomTokenFields, oauth2::basic::BasicTokenType>;
type OAuthClient<
    HasAuthUrl = EndpointNotSet,
    HasDeviceAuthUrl = EndpointNotSet,
    HasIntrospectionUrl = EndpointNotSet,
    HasRevocationUrl = EndpointNotSet,
    HasTokenUrl = EndpointNotSet,
> = oauth2::Client<
    oauth2::basic::BasicErrorResponse,
    OAuthTokenResponse,
    oauth2::basic::BasicTokenIntrospectionResponse,
    oauth2::StandardRevocableToken,
    oauth2::basic::BasicRevocationErrorResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;
