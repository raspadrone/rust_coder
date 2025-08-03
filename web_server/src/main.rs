#![allow(unused)]
use std::env;

use anyhow::{Context, Result, anyhow};
use app_core::{
    AppSettings, AppState, feedback::process_upvoted_solution, ingestion::ingest_document,
    process_query,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_extra::extract::Multipart;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

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

// This struct is for the pasted text endpoint
#[derive(Deserialize)]
struct IngestTextRequest {
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

    // Configure a permissive CORS policy for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // `GET /` goes to a simple handler
        .route("/", get(root_handler))
        // `POST /api/query` goes to our new handler
        .route("/api/query", post(api_query_handler))
        // curl --request POST http://127.0.0.1:3000/api/shutdown to stop qdrant
        .route("/api/shutdown", post(api_shutdown_handler))
        .route("/api/ingest/file", post(api_ingest_file_handler))
        .route("/api/ingest/text", post(api_ingest_text_handler))
        .route("/api/feedback", post(api_feedback_handler))
        .with_state(app_state)
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // axum allows 50 MB in bytes uploads;
        .layer(cors)
        .layer(TraceLayer::new_for_http());;

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

// /// Handler for ingesting a new document into the knowledge base
// async fn api_ingest_handler(
//     State(state): State<AppState>,
//     Json(payload): Json<IngestRequest>,
// ) -> Result<StatusCode, AppError> {
//     // We pass a clone of the state because the ingest_document function
//     // takes ownership of it.
//     ingest_document(state.clone(), payload.content).await?;
//     Ok(StatusCode::OK)
// }

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

/// Handler for ingesting from a file upload.
async fn api_ingest_file_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<StatusCode, AppError> {
    let mut document_content = String::new();

    // Explicitly handle multipart errors to provide a better response than a generic 500.
    while let Some(field) = multipart.next_field().await.map_err(|err| {
        AppError(anyhow::anyhow!(
            "Error reading multipart form data: {}",
            err
        ))
    })? {
        // Look for the specific field named "document".
        if field.name() == Some("document") {
            let content_type = field.content_type().unwrap_or("text/plain").to_string();

            // Read the raw bytes of the field first.
            let bytes = field.bytes().await.map_err(|err| {
                AppError(anyhow::anyhow!(
                    "Failed to read bytes from file field: {}",
                    err
                ))
            })?;

            // Now, process the bytes based on the content type.
            if content_type == "application/pdf" {
                document_content = pdf_extract::extract_text_from_mem(&bytes)
                    .map_err(|err| AppError(anyhow::anyhow!("Failed to extract text from PDF. The file may be corrupt or not a valid PDF. Error: {}", err)))?;
            } else {
                // Safely convert bytes to a String, handling potential non-UTF8 content.
                document_content = String::from_utf8(bytes.to_vec()).map_err(|err| {
                    AppError(anyhow::anyhow!(
                        "File content is not valid UTF-8 text: {}",
                        err
                    ))
                })?;
            }
            // Once we've found and processed the 'document' field, we can stop looking.
            break;
        }
    }

    if document_content.is_empty() {
        return Err(AppError(anyhow::anyhow!(
            "The 'document' field was not found in the request or the extracted content was empty."
        )));
    }

    // Pass the extracted text content to our core ingestion logic.
    ingest_document(state.clone(), document_content).await?;
    Ok(StatusCode::OK)
}

// New handler with extensive logging for debugging

// pub async fn api_ingest_file_handler(
//     State(state): State<AppState>,
//     mut multipart: Multipart,
// ) -> Result<StatusCode, AppError> {
//     println!("\n--- [DEBUG] Entered file ingest handler ---");

//     // Try to get the next field from the multipart stream
//     match multipart.next_field().await {
//         Ok(Some(field)) => {
//             let field_name = field.name().unwrap_or("unknown").to_string();
//             println!("[DEBUG] Found field: '{}'", field_name);

//             // Check if this is the field we expect
//             if field_name != "document" {
//                 let msg = format!("Received unexpected field: '{}'", field_name);
//                 println!("[DEBUG] ERROR: {}", msg);
//                 return Err(AppError(anyhow::anyhow!(msg)));
//             }

//             println!("[DEBUG] Field is 'document'. Proceeding to read bytes...");

//             // Try to read the bytes from the field
//             match field.bytes().await {
//                 Ok(data) => {
//                     println!("[DEBUG] Successfully read {} bytes from the field.", data.len());

//                     // --- Your original logic would go here ---
//                     // For now, we just return success.
//                     // let document_content = pdf_extract::extract_text_from_mem(&data)?;
//                     // ... etc ...

//                     println!("[DEBUG] --- Handler finished successfully ---");
//                     Ok(StatusCode::OK)
//                 }
//                 Err(e) => {
//                     // This is likely where the error is happening
//                     println!("[DEBUG] FATAL: Failed to read bytes from the field.");
//                     println!("[DEBUG] The error was: {:?}", e);
//                     Err(AppError(anyhow::anyhow!("Failed to read file bytes: {}", e)))
//                 }
//             }
//         }
//         Ok(None) => {
//             println!("[DEBUG] ERROR: No fields found in multipart request.");
//             Err(AppError(anyhow::anyhow!("No fields in multipart request.")))
//         }
//         Err(e) => {
//             println!("[DEBUG] FATAL: Error getting next field from multipart stream.");
//             println!("[DEBUG] The error was: {:?}", e);
//             Err(AppError(anyhow::anyhow!("Error processing multipart stream: {}", e)))
//         }
//     }
// }

/// Handler for ingesting from a raw text payload.
async fn api_ingest_text_handler(
    State(state): State<AppState>,
    Json(payload): Json<IngestTextRequest>,
) -> Result<StatusCode, AppError> {
    ingest_document(state.clone(), payload.content).await?;
    Ok(StatusCode::OK)
}
