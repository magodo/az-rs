use std::path::PathBuf;

pub mod invoke;
pub mod metadata_command;
pub mod metadata_index;

#[derive(Debug, Clone)]
pub struct ApiManager {
    rps: Vec<String>,
    #[allow(dead_code)]
    commands_path: PathBuf,
    #[allow(dead_code)]
    index_path: PathBuf,
}

#[cfg(any(feature = "embed-api", target_arch = "wasm32"))]
mod embedded {
    use super::metadata_command::Command;
    use super::metadata_index::Index;
    use anyhow::{anyhow, Result};
    use std::path::PathBuf;

    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "metadata/metadata/index"]
    struct IndexAsset;

    #[derive(RustEmbed)]
    #[folder = "metadata/metadata/commands"]
    struct CommandsAsset;

    impl super::ApiManager {
        pub fn new(_: PathBuf) -> Result<Self> {
            let mut rps: Vec<String> = IndexAsset::names()
                .map(|name| name.trim_end_matches(".json").to_string())
                .collect();
            rps.sort();
            Ok(Self {
                index_path: PathBuf::new(),
                commands_path: PathBuf::new(),
                rps,
            })
        }

        pub fn list_rps(&self) -> &Vec<String> {
            &self.rps
        }

        pub fn read_index(&self, rp: &str) -> Result<Index> {
            let bytes: Vec<u8> = IndexAsset::get(format!("{rp}.json").as_str())
                .map(|d| d.data.to_vec())
                .ok_or(anyhow!("{rp}.json doesn't exist"))?;
            Ok(serde_json::from_slice(&bytes)?)
        }

        pub fn read_command(&self, command_file: &str) -> Result<Command> {
            let bytes: Vec<u8> = CommandsAsset::get(command_file)
                .map(|d| d.data.to_vec())
                .ok_or(anyhow!("{command_file} doesn't exist"))?;
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}

#[cfg(not(any(feature = "embed-api", target_arch = "wasm32")))]
mod fs {
    use super::metadata_command::Command;
    use super::metadata_index::Index;
    use anyhow::{Context, Result};
    use std::fs::read;
    use std::path::PathBuf;

    impl super::ApiManager {
        pub fn new(path: PathBuf) -> Result<Self> {
            // TODO: Validate the path
            let index_path = path.join("index");
            let commands_path = path.join("commands");
            let mut rps = vec![];
            for entry in index_path.read_dir().context(format!(
                "reading index from the directory {}",
                path.display()
            ))? {
                let path = entry?.path();
                if let Some(ext) = path.extension() {
                    if ext == "json" {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            rps.push(stem.to_owned());
                        }
                    }
                }
            }
            rps.sort();
            Ok(Self {
                index_path,
                commands_path,
                rps,
            })
        }

        pub fn list_rps(&self) -> &Vec<String> {
            &self.rps
        }

        pub fn read_index(&self, rp: &str) -> Result<Index> {
            let bytes = read(self.index_path.join(format!("{rp}.json")))
                .context(format!("reading {rp}.json"))?;
            Ok(serde_json::from_slice(&bytes)?)
        }

        pub fn read_command(&self, command_file: &str) -> Result<Command> {
            let bytes = read(self.commands_path.join(command_file))
                .context(format!("reading {command_file}"))?;
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}
