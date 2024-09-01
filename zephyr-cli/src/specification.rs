use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{MercuryClient, ZephyrProjectParser};

#[derive(Deserialize, Serialize, Clone)]
pub struct Index {
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Dashboard {
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

impl ZephyrProjectParser {
    pub async fn register_indexes(&self) -> Result<()> {
        if let Some(indexes) = self.config.indexes.clone() {
            for index in indexes {
                let client = reqwest::Client::new();
                client
                    .put(format!("{}/api/indexes", self.client.base_url))
                    .header("Authorization", self.client.get_auth())
                    //.bearer_auth(self.client.jwt.clone())
                    .header("Content-Type", "application/json")
                    .body(serde_json::to_string(&index)?)
                    .send()
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn register_dashboard(&self) -> Result<()> {
        if let Some(dashboard) = self.config.dashboard.clone() {
            let client = reqwest::Client::new();
            client
                .put(format!("{}/api/dashboard", self.client.base_url))
                .header("Authorization", self.client.get_auth())
                //.bearer_auth(self.client.jwt.clone())
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&dashboard)?)
                .send()
                .await?;
        }

        Ok(())
    }
}
