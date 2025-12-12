use std::sync::Arc;

use anyhow::Result;
use azure_core::{credentials::TokenCredential, http::HttpClient};
use serde::{Deserialize, Serialize};

use crate::azidentityext::credential::{RefreshTokenSession, Session};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "session")]
pub enum AuthSession {
    RefreshTokenSession(RefreshTokenSession),
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait ProfileManager: Send + Sync + std::fmt::Debug + 'static {
    async fn load(&self) -> Result<Option<AuthSession>>;
    async fn refresh(&self, session: &AuthSession) -> Result<()>;
    async fn login(&self, session: &AuthSession) -> Result<()>;
    async fn logout(&self) -> Result<()>;

    async fn get_credential(self: Arc<Self>, http_client: Arc<dyn HttpClient>) -> Result<Option<Box<dyn TokenCredential>>> 
    where
        Self: Sized,
    {
        let auth_session = self.load().await?;
        match auth_session {
            Some(AuthSession::RefreshTokenSession(session)) => {
                let credential = session.get_credential(http_client, Some(self.clone())).await?;
                Ok(Some(Box::new(credential)))
            }
            None => Ok(None),
        }
    }
}

#[derive(Debug)]
pub struct FileSystemProfileManager {
    profile_path: std::path::PathBuf,
}

impl FileSystemProfileManager {
    pub fn new(profile_path: std::path::PathBuf) -> Arc<Self> {
        Arc::new(Self { profile_path })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl ProfileManager for FileSystemProfileManager {
    async fn load(&self) -> Result<Option<AuthSession>> {
        let profile_data = match tokio::fs::read_to_string(&self.profile_path).await {
            Ok(data) => Some(data),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        Ok(profile_data.map(|data| serde_json::from_str::<AuthSession>(&data)).transpose()?)
    }

    async fn refresh(&self, session: &AuthSession) -> Result<()> {
        self.login(session).await
    }

    async fn login(&self, session: &AuthSession) -> Result<()> {
        let session_data = serde_json::to_string(&session)?;
        tokio::fs::create_dir_all(self.profile_path.parent().unwrap()).await?;
        tokio::fs::write(&self.profile_path, session_data).await?;
        Ok(())
    }

    async fn logout(&self) -> Result<()> {
        tokio::fs::remove_file(&self.profile_path).await?;
        Ok(())
    }
}

