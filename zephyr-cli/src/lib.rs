use parser::{Column, Table};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;

use clap::{Parser, Subcommand};

mod error;
mod parser;
mod specification;

pub use parser::ZephyrProjectParser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub jwt: Option<String>,

    #[arg(short, long)]
    pub key: Option<String>,

    #[arg(short, long)]
    pub local: Option<bool>,

    #[arg(short, long)]
    pub mainnet: Option<bool>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Deploy {
        #[arg(short, long)]
        target: Option<String>,

        #[arg(short, long)]
        old_api: Option<bool>,

        #[arg(short, long)]
        force: Option<bool>,
    },

    Build,

    Catchup {
        #[arg(short, long)]
        contracts: Vec<String>,

        #[arg(short, long)]
        start: Option<i64>,

        #[arg(short, long)]
        topic1s: Option<Vec<String>>,

        #[arg(short, long)]
        topic2s: Option<Vec<String>>,

        #[arg(short, long)]
        topic3s: Option<Vec<String>>,

        #[arg(short, long)]
        topic4s: Option<Vec<String>>,
    },

    NewProject {
        #[arg(short, long)]
        name: String,
    },
}

#[derive(Deserialize, Serialize, Debug)]
struct NewZephyrTableClient {
    table: Option<String>,
    columns: Option<Vec<Column>>,
    force: bool,
}

#[derive(Deserialize, Serialize, Debug)]
struct CodeUploadClient {
    code: Option<Vec<u8>>,
    force_replace: Option<bool>,
    project_name: Option<String>,
}

pub enum MercuryAccessKey {
    Jwt(String),
    Key(String),
}

impl MercuryAccessKey {
    pub fn from_jwt(jwt: &str) -> Self {
        Self::Jwt(jwt.to_string())
    }

    pub fn from_key(key: &str) -> Self {
        Self::Key(key.to_string())
    }
}

pub struct MercuryClient {
    pub base_url: String,
    pub key: MercuryAccessKey,
}

impl MercuryClient {
    pub fn new(base_url: String, key: MercuryAccessKey) -> Self {
        Self { base_url, key }
    }

    pub fn get_auth(&self) -> String {
        match &self.key {
            MercuryAccessKey::Jwt(jwt) => format!("Bearer {}", &jwt),
            MercuryAccessKey::Key(key) => key.to_string(),
        }
    }

    pub async fn new_table(
        &self,
        table: Table,
        force: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let columns = table.columns;
        let mut cols = Vec::new();

        for col in columns {
            cols.push(Column {
                name: col.name.to_string(),
                col_type: col.col_type.to_string(),
                primary: col.primary.clone(),
                index: col.index.clone(),
            });
        }

        let code = NewZephyrTableClient {
            table: Some(table.name),
            columns: Some(cols),
            force: if force {
                force
            } else {
                if let Some(force) = table.force {
                    force
                } else {
                    false
                }
            },
        };

        let json_code = serde_json::to_string(&code)?;
        let url = format!("{}/zephyr_table_new", &self.base_url);
        let authorization = self.get_auth();

        let client = reqwest::Client::new();

        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", authorization)
            .body(json_code)
            .send()
            .await
            .unwrap();

        if response.status().is_success() {
            println!(
                "[+] Table \"{}\" created successfully",
                response.text().await.unwrap()
            );
        } else {
            println!(
                "[-] Request failed with status code: {:?}, Error: {}",
                response.status(),
                response.text().await.unwrap()
            );
        };

        Ok(())
    }

    pub async fn deploy(
        &self,
        wasm: String,
        //        force_replace: bool,
        project_name: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Reading wasm {}", wasm);
        let mut input_file = File::open(wasm)?;

        let mut buffer = Vec::new();
        input_file.read_to_end(&mut buffer)?;
        println!("(Size of program is {})", buffer.len());

        let code = CodeUploadClient {
            code: Some(buffer),
            force_replace: Some(true),
            project_name,
        };
        let json_code = serde_json::to_string(&code)?;

        let url = format!("{}/zephyr_upload", &self.base_url);
        let authorization = self.get_auth();

        let client = reqwest::Client::new();

        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", authorization)
            .body(json_code)
            .send()
            .await
            .unwrap();

        if response.status().is_success() {
            println!("[+] Deployed was successful!");
        } else {
            println!(
                "[-] Request failed with status code: {:?}",
                response.status()
            );
        };

        Ok(())
    }

    pub async fn catchup_standard(
        &self,
        contracts: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request = CatchupRequest {
            mode: ExecutionMode::EventCatchup(contracts),
        };

        self.catchup(request).await?;

        Ok(())
    }

    pub async fn catchup_scoped(
        &self,
        contracts: Vec<String>,
        topic1s: Vec<String>,
        topic2s: Vec<String>,
        topic3s: Vec<String>,
        topic4s: Vec<String>,
        start: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request = CatchupRequest {
            mode: ExecutionMode::EventCatchupScoped(ScopedEventCatchup {
                contracts,
                topic1s,
                topic2s,
                topic3s,
                topic4s,
                start,
            }),
        };

        self.catchup(request).await?;

        Ok(())
    }

    async fn catchup(&self, request: CatchupRequest) -> Result<(), Box<dyn std::error::Error>> {
        println!("Subscribing to the requested contracts.");
        self.contracts_subscribe(request.mode.clone()).await;

        let json_code = serde_json::to_string(&request)?;

        let url = format!("{}/zephyr/execute", &self.base_url);
        let authorization = self.get_auth();

        let client = reqwest::Client::new();

        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", authorization)
            .body(json_code)
            .send()
            .await
            .unwrap();

        if response.status().is_success() {
            println!(
                "Catchup request sent successfully: {}",
                response.text().await.unwrap()
            )
        } else {
            println!(
                "[-] Request failed with status code: {:?}, {}",
                response.status(),
                response.text().await.unwrap()
            );
        };

        Ok(())
    }

    async fn contracts_subscribe(&self, mode: ExecutionMode) {
        let contracts = match mode {
            ExecutionMode::EventCatchup(contracts) => contracts,
            ExecutionMode::EventCatchupScoped(ScopedEventCatchup { contracts, .. }) => contracts,
            _ => vec![], // should be unreachable anyways
        };

        let graphql_url = format!("{}/graphql", &self.base_url);
        let authorization = self.get_auth();
        let query = r#"
            query {
                allContractEventSubscriptions {
                    edges {
                        node {
                            contractId
                        }
                    }
                }
            }
        "#;

        let client = reqwest::Client::new();

        let existing_subscriptions: Result<Vec<String>, _> = client
            .post(&graphql_url)
            .header("Authorization", &authorization)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "query": query,
            }))
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .map(|json| {
                json["data"]["allContractEventSubscriptions"]["edges"]
                    .as_array()
                    .map(|edges| {
                        edges
                            .iter()
                            .filter_map(|edge| {
                                edge["node"]["contractId"].as_str().map(String::from)
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            });

        let existing_subscriptions = match existing_subscriptions {
            Ok(subs) => subs,
            Err(e) => {
                println!("Error fetching existing subscriptions: {}", e);
                vec![]
            }
        };

        for contract in contracts {
            if existing_subscriptions.contains(&contract) {
                println!("Already subscribed to events for contract: {}", contract);
                continue;
            }

            let url = format!("{}/event", &self.base_url);
            let body = serde_json::json!({ "contract_id": contract });

            match client
                .post(&url)
                .header("Authorization", &authorization)
                .json(&body)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        println!(
                            "Successfully subscribed to events for contract: {}",
                            contract
                        );
                    } else {
                        println!(
                            "Failed to subscribe to events for contract: {}. Status: {:?}",
                            contract,
                            response.status()
                        );
                    }
                }
                Err(e) => println!(
                    "Error subscribing to events for contract {}: {}",
                    contract, e
                ),
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InvokeZephyrFunction {
    fname: String,
    arguments: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScopedEventCatchup {
    contracts: Vec<String>,
    topic1s: Vec<String>,
    topic2s: Vec<String>,
    topic3s: Vec<String>,
    topic4s: Vec<String>,
    start: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ExecutionMode {
    EventCatchup(Vec<String>),
    EventCatchupScoped(ScopedEventCatchup),
    Function(InvokeZephyrFunction),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CatchupRequest {
    mode: ExecutionMode,
}
