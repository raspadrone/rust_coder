#![allow(unused)]
use std::env;

use anyhow::{Context, Result, anyhow};
use app_core::{feedback::process_upvoted_solution, ingestion::ingest_document, process_query, AppSettings, AppState};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};

use crate::docker_manager::stop_and_remove_qdrant;

pub mod docker_manager;

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

#[derive(Deserialize)]
struct IngestRequest {
    content: String,
}

#[derive(Deserialize)]
struct FeedbackRequest {
    query: String,
    code: String,
    upvoted: bool,
}

/*-------------------------------------- main -----------------------------------------*/

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let docker_host = env::var("DOCKER_HOST").expect("`.env` must contain DOCKER_HOST");
    // qdrant start
    docker_manager::ensure_qdrant_running(docker_host)
        .await
        .context("Failed to ensure Qdrant container is running")?;
    let gemini_key = env::var("GEMINI_API_KEY").expect("`.env` must contain GEMINI_API_KEY");
    // OPENAI KEY
    let openai_key = env::var("OPENAI_API_KEY").expect("`.env` must contain OPENAI_API_KEY");
    // gemini model
    let gemini_model = env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".to_owned());
    // openai model
    let openai_model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt.4o-mini".to_owned());

    let settings = AppSettings::new().map_err(|e| anyhow!("Failed to load settings.Error: {e}"))?;
    let mut app_state = AppState::new(settings)
        .await
        .context("Failed to initialize app state.")?;
    /**********************choose model ***********/
    app_state.model = gemini_model;

    let app = Router::new()
        // `GET /` goes to a simple handler
        .route("/", get(root_handler))
        // `POST /api/query` goes to our new handler
        .route("/api/query", post(api_query_handler))
        // curl --request POST http://127.0.0.1:3000/api/shutdown to stop qdrant
        .route("/api/shutdown", post(api_shutdown_handler))
        .route("/api/ingest", post(api_ingest_handler))
        .route("/api/feedback", post(api_feedback_handler))
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

/// Handler for stopping and removing the Qdrant container
async fn api_shutdown_handler() -> Result<StatusCode, AppError> {
    dotenv().ok();
    let docker_host = env::var("DOCKER_HOST").expect("`.env` must contain DOCKER_HOST");
    stop_and_remove_qdrant(docker_host)
        .await
        .context("Failed to stop and remove Qdrant container")?;
    Ok(StatusCode::OK)
}

/// Handler for ingesting a new document into the knowledge base
async fn api_ingest_handler(
    State(state): State<AppState>,
    Json(payload): Json<IngestRequest>,
) -> Result<StatusCode, AppError> {
    // We pass a clone of the state because the ingest_document function
    // takes ownership of it.
    ingest_document(state.clone(), payload.content).await?;
    Ok(StatusCode::OK)
}

/// Handler for receiving feedback on a generated solution.
async fn api_feedback_handler(
    State(state): State<AppState>,
    Json(payload): Json<FeedbackRequest>,
) -> Result<StatusCode, AppError> {
    if payload.upvoted {
        // Pass a reference to the state
        process_upvoted_solution(&state, payload.query, payload.code).await?;
    }
    // For now, we do nothing on a downvote
    Ok(StatusCode::OK)
}