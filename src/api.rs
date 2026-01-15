use axum::extract::{Json, State};
use axum::response::{IntoResponse, Response};
use axum::{Router, routing::post};
use http::status::StatusCode;
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}

impl IntoResponse for crate::Error {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: self.to_string(),
            }),
        )
            .into_response()
    }
}

#[axum::debug_handler]
async fn token_endpoint(
    State(config): State<Arc<Config>>,
    Json(request): Json<TokenRequest>,
) -> Result<Json<TokenResponse>, crate::Error> {
    crate::exchange(&request.token, &config)
        .await
        .map(|token| Json(TokenResponse { token }))
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(&config.listen).await?;
    let app = Router::new()
        .route("/_trusted-publisher/token", post(token_endpoint))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(Arc::new(config));
    axum::serve(listener, app).await?;
    Ok(())
}
