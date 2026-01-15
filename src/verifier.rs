use crate::Error;
use crate::config::{Config, Policy};
use anyhow::Context;
use jsonwebtoken::Algorithm;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::TokenData;
use jsonwebtoken::Validation;
use jsonwebtoken::jwk::{Jwk, JwkSet, KeyAlgorithm};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct UnverifiedClaims {
    pub iss: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Claims {
    #[serde(default)]
    pub iss: Option<String>,
    #[serde(default)]
    pub sub: Option<String>,

    // For better or worse, jwcrypto already requires an `exp` claim by default anyway so no need
    // to make this an Option.
    pub exp: u64,

    #[serde(flatten)]
    other: HashMap<String, serde_json::Value>,
}

impl Claims {
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        match key {
            "iss" => self.iss.clone().map(serde_json::Value::from),
            "sub" => self.sub.clone().map(serde_json::Value::from),
            "exp" => Some(serde_json::Value::from(self.exp)),
            _ => self.other.get(key).cloned(),
        }
    }
}

impl UnverifiedClaims {
    pub fn decode(token: &str) -> jsonwebtoken::errors::Result<TokenData<Self>> {
        jsonwebtoken::dangerous::insecure_decode(token)
    }
}

#[derive(Debug, serde::Deserialize)]
struct OpenIdConfig {
    jwks_uri: String,
}

pub async fn load_jwks(issuer: &str) -> anyhow::Result<JwkSet> {
    let url = format!("{}/.well-known/openid-configuration", issuer);
    let config: OpenIdConfig = reqwest::get(url).await?.error_for_status()?.json().await?;
    let jwks: JwkSet = reqwest::get(config.jwks_uri)
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(jwks)
}

pub async fn resolve_key(issuer: &str, kid: &str) -> anyhow::Result<Jwk> {
    let jwks = load_jwks(issuer).await?;
    jwks.find(kid)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Unknown key"))
}

pub fn check_claims(policy: &Policy, claims: &Claims) -> Result<(), Error> {
    if policy.required_claims.is_empty() {
        return Err(Error::EmptyPolicyClaims);
    }

    if claims.iss.as_deref() != Some(&policy.issuer) {
        panic!("issuer is invalid - this should not be possible");
    }

    for (key, expected) in &policy.required_claims {
        match claims.get(key) {
            Some(actual) => {
                if actual != *expected {
                    return Err(Error::InvalidClaim { claim: key.into() });
                }
            }
            None => {
                return Err(Error::InvalidClaim { claim: key.into() });
            }
        }
    }
    Ok(())
}

pub fn key_algorithm(key: &Jwk) -> anyhow::Result<Algorithm> {
    match key.common.key_algorithm {
        Some(KeyAlgorithm::HS256) => Ok(Algorithm::HS256),
        Some(KeyAlgorithm::HS384) => Ok(Algorithm::HS384),
        Some(KeyAlgorithm::HS512) => Ok(Algorithm::HS512),
        Some(KeyAlgorithm::ES256) => Ok(Algorithm::ES256),
        Some(KeyAlgorithm::ES384) => Ok(Algorithm::ES384),
        Some(KeyAlgorithm::RS256) => Ok(Algorithm::RS256),
        Some(KeyAlgorithm::RS384) => Ok(Algorithm::RS384),
        Some(KeyAlgorithm::RS512) => Ok(Algorithm::RS512),
        Some(KeyAlgorithm::PS256) => Ok(Algorithm::PS256),
        Some(KeyAlgorithm::PS384) => Ok(Algorithm::PS384),
        Some(KeyAlgorithm::PS512) => Ok(Algorithm::PS512),
        Some(KeyAlgorithm::EdDSA) => Ok(Algorithm::EdDSA),
        Some(_) => anyhow::bail!("Unsupported key algorithm"),
        None => anyhow::bail!("Key does not specify an algorithm"),
    }
}

pub async fn verify<'a>(token: &str, config: &'a Config) -> Result<(Claims, &'a Policy), Error> {
    let unverified_token = UnverifiedClaims::decode(&token).context("Cannot decode token")?;
    let Some(candidate_policies) = config.policies.get(&unverified_token.claims.iss) else {
        return Err(Error::InvalidClaim {
            claim: "iss".into(),
        });
    };
    let Some(kid) = unverified_token.header.kid else {
        return Err(Error::Other(anyhow::anyhow!(
            "Token header does not have a key ID"
        )));
    };
    let key = resolve_key(&unverified_token.claims.iss, &kid).await?;

    let mut validation = Validation::new(key_algorithm(&key)?);
    validation.set_audience(&[&config.audience]);
    validation.required_spec_claims.insert("exp".into());
    validation.required_spec_claims.insert("aud".into());
    validation.validate_aud = true;
    validation.validate_exp = true;
    validation.validate_nbf = true;

    let decoding_key = DecodingKey::from_jwk(&key).map_err(anyhow::Error::from)?;
    let decoded: TokenData<Claims> = jsonwebtoken::decode(token, &decoding_key, &validation)
        .context("while decoding ID token")?;

    let mut errors = Vec::new();
    for policy in candidate_policies {
        match check_claims(&policy, &decoded.claims) {
            Ok(()) => return Ok((decoded.claims, policy)),
            Err(err) => errors.push(err),
        }
    }

    Err(Error::NoValidPolicy(errors).into())
}
