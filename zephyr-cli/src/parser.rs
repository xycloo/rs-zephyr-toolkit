use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufRead, Read},
    path::Path,
    process::Command,
};

use crate::{
    error::ParserError,
    specification::{Dashboard, Index},
    MercuryClient,
};

impl Config {
    fn tables(&self) -> Vec<Table> {
        self.tables.clone().unwrap_or(vec![])
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub name: String,

    /// Optionally set the project name.
    pub project: Option<String>,

    /// Tables that the poject is writing or reading.
    pub tables: Option<Vec<Table>>,

    /// Declared public indexes to register.
    pub indexes: Option<Vec<Index>>,

    /// Declared dashboard (if any) to register.
    pub dashboard: Option<Dashboard>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub force: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Column {
    pub name: String,
    pub col_type: String,
    pub primary: Option<bool>,
    pub index: Option<bool>,
}

pub struct ZephyrProjectParser {
    pub(crate) config: Config,
    pub(crate) client: MercuryClient,
}

impl ZephyrProjectParser {
    pub fn from_path<P: AsRef<Path>>(client: MercuryClient, path: P) -> Result<Self> {
        let project_definition = {
            let mut content = String::new();
            File::open(path)?.read_to_string(&mut content)?;

            content
        };

        let parser = Self {
            client,
            config: toml::from_str(&project_definition)?,
        };

        Ok(parser)
    }

    pub fn build_wasm(&self) -> Result<()> {
        let mut child = Command::new("cargo")
            .args(&["build", "--release", "--target=wasm32-unknown-unknown"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_thread = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    println!("{}", line);
                }
            }
        });

        let stderr_thread = std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("{}", line);
                }
            }
        });

        let status = child.wait()?;
        stdout_thread.join().unwrap();
        stderr_thread.join().unwrap();

        if !status.success() {
            return Err(ParserError::WasmBuildError("Build failed".to_string()).into());
        }

        Ok(())
    }

    pub async fn deploy_tables(&self, force: bool) -> Result<()> {
        for table in self.config.tables() {
            if let Err(_) = self.client.new_table(table, force).await {
                return Err(ParserError::TableCreationError.into());
            };
        }

        Ok(())
    }

    pub async fn deploy_wasm(&self, target: Option<String>) -> Result<()> {
        let project_name = &self.config.name;
        let path = if let Some(target_dir) = target {
            format!("{}/{}.wasm", target_dir, project_name.replace('-', "_"))
        } else {
            format!(
                "./target/wasm32-unknown-unknown/release/{}.wasm",
                project_name.replace('-', "_")
            )
        };

        let project_name = if let Some(pname) = self.config.project.clone() {
            pname
        } else {
            self.config.name.clone()
        };

        if let Err(_) = self.client.deploy(path, Some(project_name), None).await {
            return Err(ParserError::WasmDeploymentError.into());
        };

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{Column, Config, Table};

    #[test]
    pub fn sample_config() {
        let config = Config {
            name: "zephyr-soroban-op-ratio".into(),
            project: None,
            indexes: None,
            dashboard: None,
            tables: Some(vec![Table {
                name: "opratio".into(),
                columns: vec![
                    Column {
                        name: "soroban".into(),
                        col_type: "BYTEA".into(), // only supported type as of now
                    },
                    Column {
                        name: "ratio".into(),
                        col_type: "BYTEA".into(), // only supported type as of now
                    },
                ],
            }]),
        };

        println!("{}", toml::to_string(&config).unwrap());
    }
}
