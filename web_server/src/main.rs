use app_core::process_query;
use axum::{
    Json, Router,
    routing::{get, post},
};

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
}

#[derive(Serialize)]
struct QueryResponse {
    response: String,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        // `GET /` goes to a simple handler
        .route("/", get(root_handler))
        // `POST /api/query` goes to our new handler
        .route("/api/query", post(api_query_handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

/// A simple handler for the root path
async fn root_handler() -> &'static str {
    "Welcome to the Rust Coder API"
}

/// The main handler for our API. It accepts a JSON payload
/// and returns a JSON response.
async fn api_query_handler(
    // This tells axum to deserialize the request body as JSON into our struct
    Json(payload): Json<QueryRequest>,
) -> Json<QueryResponse> {
    // Call the processing function from our core library
    let response_string = process_query(&payload.query);
    // For now, just construct a response and send it back
    let response = QueryResponse {
        response: format!("Received your query: '{}'", response_string),
    };

    // `Json` will automatically serialize our struct into a JSON response
    // and set the correct `Content-Type` header
    Json(response)
}
