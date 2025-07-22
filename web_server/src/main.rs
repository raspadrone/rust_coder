use anyhow::{Context, Result, anyhow};
use app_core::{AppSettings, AppState, process_query};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

/*-------------------------------------- models -----------------------------------------*/

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
}

#[derive(Serialize)]
struct QueryResponse {
    response: String,
}

// app error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into `Result<_, AppError>`.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
/*-------------------------------------- main -----------------------------------------*/

#[tokio::main]
async fn main() -> Result<()> {
    let settings = AppSettings::new().map_err(|e| anyhow!("Failed to load settings.Error: {e}"))?;
    let app_state = AppState::new(settings)
        .await
        .context("Failed to initialize app state.")?;
    let app = Router::new()
        // `GET /` goes to a simple handler
        .route("/", get(root_handler))
        // `POST /api/query` goes to our new handler
        .route("/api/query", post(api_query_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

/*-------------------------------------- handlers -----------------------------------------*/

/// A simple handler for the root path
async fn root_handler() -> &'static str {
    "Welcome to the Rust Coder API"
}

/// The main handler for our API. It accepts a JSON payload
/// and returns a JSON response.
async fn api_query_handler(
    State(state): State<AppState>,
    Json(payload): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, AppError> {
    // Call processing function from core library
    let response_string = process_query(&payload.query, &state).await?;
    let response = QueryResponse {
        response: format!("Received your query: '{}'", response_string),
    };
    Ok(Json(response))
}


