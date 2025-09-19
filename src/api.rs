use metadata_index::Index;
use std::path::PathBuf;
pub mod invoke;
pub mod metadata_command;
pub mod metadata_index;

#[derive(Debug, Clone)]
pub struct ApiManager {
    pub index: Index,
    #[allow(dead_code)]
    commands_path: PathBuf,
}

#[cfg(any(feature = "embed-api", target_arch = "wasm32"))]
mod embedded {
    use super::metadata_command::Command;
    use anyhow::{anyhow, Result};
    use std::path::PathBuf;

    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "metadata/metadata"]
    struct Asset;

    impl super::ApiManager {
        pub fn new(_: PathBuf) -> Result<Self> {
            let bytes: Vec<u8> = Asset::get("index.json")
                .map(|d| d.data.to_vec())
                .ok_or(anyhow!("index.json doesn't exist"))?;
            let index = serde_json::from_slice(&bytes)?;

            Ok(Self {
                index,
                commands_path: PathBuf::new(),
            })
        }

        pub fn read_command(&self, command_file: &str) -> Result<Command> {
            let bytes: Vec<u8> = Asset::get(format!("commands/{}", command_file).as_str())
                .map(|d| d.data.to_vec())
                .ok_or(anyhow!("{command_file} doesn't exist"))?;
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}

#[cfg(not(any(feature = "embed-api", target_arch = "wasm32")))]
mod fs {
    use super::metadata_command::Command;
    use anyhow::{Context, Result};
    use std::fs::read;
    use std::path::PathBuf;

    impl super::ApiManager {
        pub fn new(path: PathBuf) -> Result<Self> {
            // TODO: Validate the files
            let index_path = path.join("index.json");
            let commands_path = path.join("commands");
            let bytes = read(index_path).context(format!("reading the index file"))?;
            let index = serde_json::from_slice(&bytes)?;
            Ok(Self {
                index,
                commands_path,
            })
        }

        pub fn read_command(&self, command_file: &str) -> Result<Command> {
            let bytes = read(self.commands_path.join(command_file))
                .context(format!("reading {command_file}"))?;
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}
