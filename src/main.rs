use crate::config::Config;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod api;
mod client;
mod config;
mod token;
mod verifier;

#[derive(Subcommand)]
enum Command {
    Exchange {
        token: String,

        #[arg(long)]
        config: PathBuf,
    },
    API {
        #[arg(long, default_value = "[::]:3000")]
        listen: String,

        #[arg(long)]
        config: PathBuf,
    },
    Login {
        url: String,
        token: Option<String>,
    },
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid or missing claim `{claim}`")]
    InvalidClaim { claim: String },

    #[error("policy has empty required claims")]
    EmptyPolicyClaims,

    #[error("token did not match any registered policy {0:?}")]
    NoValidPolicy(Vec<Error>),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

async fn exchange(token: &str, config: &Config) -> Result<String, Error> {
    let (claims, policy) = crate::verifier::verify(&token, &config).await?;
    token::issue(&claims, policy, config)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let cli = Cli::parse();
    match cli.command {
        Command::Exchange { token, config } => {
            let config = {
                let contents = std::fs::read_to_string(&config)?;
                toml::from_str::<Config>(&contents)?
            };
            println!("{}", exchange(&token, &config).await?);
        }
        Command::API { listen, config } => {
            let config = {
                let contents = std::fs::read_to_string(&config)?;
                toml::from_str::<Config>(&contents)?
            };
            api::run(&listen, config).await?;
        }
        Command::Login { url, token } => {
            println!("{}", client::login(&url, token.as_deref()).await?);
        }
    }

    Ok(())
}
