use crate::api::{TokenRequest, TokenResponse};
use anyhow::Context;
use serde::Deserialize;

async fn github_login(url: &str, token: &str, audience: &str) -> anyhow::Result<String> {
    #[derive(Deserialize)]
    struct Response {
        value: String,
    }

    let client = reqwest::Client::new();
    let url = reqwest::Url::parse_with_params(url, &[("audience", audience)])?;
    let response: Response = client
        .get(url.clone())
        .bearer_auth(token)
        .send()
        .await
        .with_context(|| format!("while sending request to GitHub OIDC endpoint: {}", url))?
        .error_for_status()
        .with_context(|| format!("HTTP error from GitHub OIDC endpoint: {}", url))?
        .json()
        .await
        .with_context(|| format!("while reading response from GitHub OIDC endpoint: {}", url))?;

    Ok(response.value)
}

async fn get_oidc_token(audience: &str) -> anyhow::Result<String> {
    if let Ok(token) = std::env::var("ATTIC_TRUSTED_PUBLISHER_ID_TOKEN") {
        return Ok(token);
    }

    if let Ok(url) = std::env::var("ACTIONS_ID_TOKEN_REQUEST_URL")
        && let Ok(token) = std::env::var("ACTIONS_ID_TOKEN_REQUEST_TOKEN")
    {
        return github_login(&url, &token, audience)
            .await
            .context("while fetching OIDC token from GitHub");
    }

    anyhow::bail!("Could not find OIDC token in environment");
}

pub async fn login(url: &str, token: Option<&str>) -> anyhow::Result<String> {
    let token = if let Some(t) = token {
        t.to_owned()
    } else {
        get_oidc_token(url).await?
    };

    let client = reqwest::Client::new();
    let endpoint = format!("{url}/_trusted-publisher/token");
    let response: TokenResponse = client
        .post(endpoint.clone())
        .json(&TokenRequest { token })
        .send()
        .await
        .with_context(|| format!("while sending token exchange request to {endpoint}"))?
        .error_for_status()
        .with_context(|| format!("HTTP status error from token exchange request to {endpoint}"))?
        .json()
        .await
        .with_context(|| format!("while reading token exchange response from {endpoint}"))?;
    Ok(response.token)
}
