use axum::extract::{Json, State};
use axum::{Router, routing::post};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::Config;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenRequest {
    pub token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
}

#[axum::debug_handler]
async fn token_endpoint(
    State(config): State<Arc<Config>>,
    Json(request): Json<TokenRequest>,
) -> Json<TokenResponse> {
    let token = crate::exchange(&request.token, &config).await.unwrap();
    println!("Issuing token: {token}");
    Json(TokenResponse { token: token })
}

pub async fn run(listen: &str, config: Config) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/_trusted-publisher/token", post(token_endpoint))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(Arc::new(config));
    let listener = tokio::net::TcpListener::bind(listen).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
