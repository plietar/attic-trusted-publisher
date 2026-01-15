use crate::api::{ApiError, TokenRequest, TokenResponse};
use anyhow::Context;
use anyhow::bail;
use mime::Mime;
use serde::Deserialize;

trait CheckError: Sized {
    async fn check_error(self) -> anyhow::Result<Self>;
}
impl CheckError for reqwest::Response {
    async fn check_error(self) -> anyhow::Result<Self> {
        let status = self.status();
        if status.is_client_error() || status.is_server_error() {
            let is_json = self
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<Mime>().ok())
                .map(|value| value.essence_str() == "application/json")
                .unwrap_or(false);

            if is_json {
                let body: ApiError = self.json().await?;
                bail!("{status}: {}", body.error);
            } else {
                bail!("{status}: {}", self.text().await?);
            }
        } else {
            Ok(self)
        }
    }
}

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
        .check_error()
        .await
        .with_context(|| format!("while sending token exchange request to {endpoint}"))?
        .json()
        .await
        .with_context(|| format!("while reading token exchange response from {endpoint}"))?;
    Ok(response.token)
}
