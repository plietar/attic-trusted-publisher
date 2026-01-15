use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use itertools::Itertools;
use jsonwebtoken::{Algorithm, EncodingKey};
use serde::de;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, better_default::Default)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct Permissions {
    #[default(false)]
    pub pull: bool,
    #[default(false)]
    pub push: bool,
    #[default(false)]
    pub delete: bool,
    #[default(false)]
    pub create_cache: bool,
    #[default(false)]
    pub configure_cache: bool,
    #[default(false)]
    pub configure_cache_retention: bool,
    #[default(false)]
    pub destroy_cache: bool,
}

fn get_false() -> bool {
    false
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    #[serde(with = "humantime_serde")]
    #[serde(default)]
    pub duration: Option<Duration>,
    pub issuer: String,
    pub permissions: HashMap<String, Permissions>,

    #[serde(default = "get_false")]
    pub allow_extending_token_lifespan: bool,

    pub required_claims: HashMap<String, serde_json::Value>,
}

fn deserialize_policies<'de, D>(d: D) -> Result<HashMap<String, Vec<Policy>>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let policies: Vec<Policy> = Deserialize::deserialize(d)?;
    Ok(policies.into_iter().into_group_map_by(|p| p.issuer.clone()))
}

const ENV_TOKEN_HS256_SECRET_BASE64: &str = "ATTIC_SERVER_TOKEN_HS256_SECRET_BASE64";
const ENV_TOKEN_RS256_SECRET_BASE64: &str = "ATTIC_SERVER_TOKEN_RS256_SECRET_BASE64";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub audience: String,

    #[serde(deserialize_with = "deserialize_policies")]
    pub policies: HashMap<String, Vec<Policy>>,

    #[serde(default)]
    pub jwt: JWTConfig,
}

#[derive(Clone, derive_more::Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JWTConfig {
    #[serde(default = "load_jwt_signing_config_from_env")]
    #[debug(skip)]
    pub signing: JWTSigningConfig,

    #[serde(rename = "token-bound-issuer")]
    #[serde(default)]
    pub token_bound_issuer: Option<String>,

    #[serde(rename = "token-bound-audiences")]
    #[serde(default)]
    pub token_bound_audiences: Option<HashSet<String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub enum JWTSigningConfig {
    #[serde(rename = "token-rs256-secret-base64")]
    #[serde(deserialize_with = "deserialize_token_rsa_secret_base64")]
    RS256SignAndVerify(EncodingKey),

    #[serde(rename = "token-hs256-secret-base64")]
    #[serde(deserialize_with = "deserialize_token_hmac_secret_base64")]
    HS256SignAndVerify(EncodingKey),
}

impl JWTSigningConfig {
    pub fn key(&self) -> &EncodingKey {
        match self {
            JWTSigningConfig::RS256SignAndVerify(key)
            | JWTSigningConfig::HS256SignAndVerify(key) => key,
        }
    }

    pub fn alg(&self) -> Algorithm {
        match self {
            JWTSigningConfig::RS256SignAndVerify(_) => Algorithm::RS256,
            JWTSigningConfig::HS256SignAndVerify(_) => Algorithm::HS256,
        }
    }
}

impl Default for JWTConfig {
    fn default() -> JWTConfig {
        JWTConfig {
            token_bound_issuer: None,
            token_bound_audiences: None,
            signing: load_jwt_signing_config_from_env(),
        }
    }
}

fn read_non_empty_var(key: &str) -> anyhow::Result<Option<String>> {
    let value = match env::var(key) {
        Err(env::VarError::NotPresent) => {
            return Ok(None);
        }
        r => r?,
    };

    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn load_token_hs256_secret_from_env() -> Option<JWTSigningConfig> {
    let s = read_non_empty_var(ENV_TOKEN_HS256_SECRET_BASE64)
        .expect("HS256 environment cannot be read")?;

    let secret = decode_token_hmac_secret_base64(&s).expect("HS256 secret cannot be decoded");

    Some(JWTSigningConfig::HS256SignAndVerify(secret))
}

fn load_token_rs256_secret_from_env() -> Option<JWTSigningConfig> {
    let s = read_non_empty_var(ENV_TOKEN_RS256_SECRET_BASE64)
        .expect("RS256 environment cannot be read")?;

    let secret = decode_token_rsa_secret_base64(&s).expect("RS256 cannot be decoded");

    Some(JWTSigningConfig::RS256SignAndVerify(secret))
}

fn load_jwt_signing_config_from_env() -> JWTSigningConfig {
    if let Some(config) = load_token_rs256_secret_from_env() {
        config
    } else if let Some(config) = load_token_hs256_secret_from_env() {
        config
    } else {
        panic!("Missing JWT signing configuration");
    }
}

fn deserialize_token_hmac_secret_base64<'de, D>(deserializer: D) -> Result<EncodingKey, D::Error>
where
    D: de::Deserializer<'de>,
{
    use de::Error;

    let s = String::deserialize(deserializer)?;
    let key = decode_token_hmac_secret_base64(&s).map_err(Error::custom)?;

    Ok(key)
}

fn deserialize_token_rsa_secret_base64<'de, D>(deserializer: D) -> Result<EncodingKey, D::Error>
where
    D: de::Deserializer<'de>,
{
    use de::Error;

    let s = String::deserialize(deserializer)?;
    let key = decode_token_rsa_secret_base64(&s).map_err(Error::custom)?;

    Ok(key)
}

pub fn decode_token_hmac_secret_base64(s: &str) -> anyhow::Result<EncodingKey> {
    Ok(EncodingKey::from_base64_secret(s)?)
}

pub fn decode_token_rsa_secret_base64(s: &str) -> anyhow::Result<EncodingKey> {
    let decoded = BASE64_STANDARD.decode(s)?;
    Ok(EncodingKey::from_rsa_pem(&decoded)?)
}
