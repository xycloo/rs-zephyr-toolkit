use std::{
    fs::{File, OpenOptions},
    io::Write,
};

use clap::Parser;
use mercury_cli::{Cli, Commands, MercuryClient, ZephyrProjectParser};

const BACKEND_ENDPOINT: &str = "https://api.mercurydata.app";
const MAINNET_BACKEND_ENDPOINT: &str = "https://mainnet.mercurydata.app";
const LOCAL_BACKEND: &str = "http://127.0.0.1:8443";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let client = if let Some(true) = cli.local {
        MercuryClient::new(LOCAL_BACKEND.to_string(), cli.jwt.unwrap_or("".into()))
    } else {
        if let Some(true) = cli.mainnet {
            MercuryClient::new(
                MAINNET_BACKEND_ENDPOINT.to_string(),
                cli.jwt.unwrap_or("".into()),
            )
        } else {
            MercuryClient::new(BACKEND_ENDPOINT.to_string(), cli.jwt.unwrap_or("".into()))
        }
    };

    match cli.command {
        Some(Commands::Deploy {
            target,
            old_api,
            force,
        }) => {
            if let Some(true) = old_api {
                println!("Deploying wasm ...");
                client
                    .deploy(target.unwrap(), force.unwrap_or(false))
                    .await
                    .unwrap();
                println!("Successfully deployed Zephyr program.");
            } else {
                println!("Parsing project configuration ...");
                let parser = ZephyrProjectParser::from_path(client, "./zephyr.toml").unwrap();
                println!("Building binary ...");
                parser.build_wasm().unwrap();
                println!("Deploying tables ...");
                parser.deploy_tables().await.unwrap();

                println!("Deploying wasm ...");
                parser.deploy_wasm(target).await.unwrap();

                println!("Successfully deployed Zephyr program.");
            }
        }

        Some(Commands::Catchup { contracts }) => {
            if client.catchup(contracts).await.is_err() {
                println!("Catchup request failed client-side.")
            }
        }

        Some(Commands::NewProject { name }) => {
            let output = std::process::Command::new("cargo")
                .args(&["new", "--lib", &name])
                .output()?;

            if !output.status.success() {
                println!("Failed to create new project")
            }

            let output = std::process::Command::new("touch")
                .args(&[&format!("{}/zephyr.toml", name)])
                .output()?;

            if !output.status.success() {
                println!("Failed to create new project")
            }

            let output = std::process::Command::new("mkdir")
                .args(&[&format!("{}/.cargo", name)])
                .output()?;

            if !output.status.success() {
                println!("Failed to create new project")
            }

            let output = std::process::Command::new("touch")
                .args(&[&format!("{}/.cargo/config", name)])
                .output()?;

            if !output.status.success() {
                println!("Failed to create new project")
            }

            let mut toml = File::create(format!("{}/zephyr.toml", name))?;
            toml.write_all(format!(r#"name = "{}""#, name).as_bytes())?;
            toml.flush()?;

            let mut config = File::create(format!("{}/.cargo/config", name))?;
            config.write_all(
                r#"[target.wasm32-unknown-unknown]
rustflags = [
    "-C", "target-feature=+multivalue",
    "-C", "link-args=-z stack-size=10000000",
]
            "#
                .as_bytes(),
            )?;
            config.flush()?;

            let starter = r#"use zephyr_sdk::{prelude::*, EnvClient};

#[no_mangle]
pub extern "C" fn on_close() {
    let env = EnvClient::new();
}            
"#
            .as_bytes();

            let mut lib = File::create(format!("{}/src/lib.rs", name))?;
            lib.write_all(starter)?;
            lib.flush()?;

            let mut cargo_toml = OpenOptions::new()
                .append(true)
                .open(format!("{}/Cargo.toml", name))?;
            cargo_toml.write(
                r#"zephyr-sdk = { version = "0.1.6" }

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "z"
overflow-checks = true
debug = 0
strip = "symbols"
debug-assertions = false
panic = "abort"
codegen-units = 1
lto = true
"#
                .as_bytes(),
            )?;
        }

        None => {
            println!("Usage: zephyr deploy")
        }
    };

    Ok(())
}
