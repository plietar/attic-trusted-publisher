use crate::config::Config;
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

mod api;
mod client;
mod config;
mod verifier;

fn is_false(v: &bool) -> bool {
    *v == false
}

use serde_with::{BoolFromInt, serde_as};
#[serde_as]
#[derive(Clone, Debug, Serialize)]
pub struct AtticCachePermissions {
    #[serde(rename = "r")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde_as(as = "BoolFromInt")]
    pull: bool,

    #[serde(rename = "w")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde_as(as = "BoolFromInt")]
    push: bool,

    #[serde(rename = "d")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde_as(as = "BoolFromInt")]
    delete: bool,

    #[serde(rename = "cc")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde_as(as = "BoolFromInt")]
    create_cache: bool,

    #[serde(rename = "cr")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde_as(as = "BoolFromInt")]
    configure_cache: bool,

    #[serde(rename = "cq")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde_as(as = "BoolFromInt")]
    configure_cache_retention: bool,

    #[serde(rename = "cd")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde_as(as = "BoolFromInt")]
    destroy_cache: bool,
}

#[derive(Clone, Debug, Serialize)]
struct AtticClaim {
    caches: HashMap<String, AtticCachePermissions>,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize)]
struct AtticTokenBody {
    #[serde(rename = "https://jwt.attic.rs/v1")]
    attic: AtticClaim,

    sub: Option<String>,
    iss: Option<String>,
    aud: Option<HashSet<String>>,
    exp: Option<u64>,
    iat: u64,
}

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

pub fn reduce<T, U, R, F>(left: Option<T>, right: Option<U>, f: F) -> Option<R>
where
    T: Into<R>,
    U: Into<R>,
    F: FnOnce(T, U) -> R,
{
    match (left, right) {
        (Some(l), Some(r)) => Some(f(l, r)),
        (Some(l), None) => Some(l.into()),
        (None, Some(r)) => Some(r.into()),
        (None, None) => None,
    }
}

async fn exchange(token: &str, config: &Config) -> anyhow::Result<String> {
    let (claims, policy) = crate::verifier::verify(&token, &config).await?;

    let iat = jsonwebtoken::get_current_timestamp();
    let exp = if policy.allow_extending_token_lifespan {
        policy.duration.map(|t| iat + t.as_secs())
    } else {
        reduce(
            policy.duration.map(|t| iat + t.as_secs()),
            claims.exp,
            std::cmp::min,
        )
    };

    let permissions = policy
        .permissions
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                AtticCachePermissions {
                    pull: v.pull,
                    push: v.push,
                    delete: v.delete,
                    create_cache: v.create_cache,
                    configure_cache: v.configure_cache,
                    configure_cache_retention: v.configure_cache_retention,
                    destroy_cache: v.destroy_cache,
                },
            )
        })
        .collect();

    let body = AtticTokenBody {
        sub: claims.sub,
        exp,
        iat,
        iss: config.jwt.token_bound_issuer.clone(),
        aud: config.jwt.token_bound_audiences.clone(),
        attic: AtticClaim {
            caches: permissions,
        },
    };
    let result = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(config.jwt.signing.alg()),
        &body,
        config.jwt.signing.key(),
    )?;

    Ok(result)
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
