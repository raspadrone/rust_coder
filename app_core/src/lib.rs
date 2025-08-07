#![allow(unused)]
use std::sync::Arc;

use anyhow::{Context, Result};
use config::{Config, File};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use genai::Client;
use ort::{execution_providers::CUDAExecutionProvider, session::Session};
use qdrant_client::Qdrant;
use serde::Deserialize;

use crate::{ sandbox::run_in_sandbox, web_search::search_and_scrape};

pub mod feedback;
pub mod ingestion;
pub mod llm;
pub mod qdrant;
pub mod sandbox;
pub mod web_scraper;
pub mod web_search;

#[derive(Deserialize, Clone)]
pub struct AppSettings {
    pub qdrant_url: String,
    pub llm_model: String,
}

impl AppSettings {
    /// Loads the application settings from the configuration file.
    pub fn new() -> Result<Self> {
        let s = Config::builder()
            .add_source(File::with_name("config/default"))
            .build()?;
        let settings = s.try_deserialize()?;
        Ok(settings)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub qdrant_client: Qdrant,
    pub genai_client: Client,
    pub model: String,
    pub embedding_model: Arc<TextEmbedding>,
    pub http_client: Arc<reqwest::Client>, // for scraping
}

impl AppState {
    /// Initializes the application state, connecting to required services.
    pub async fn new(settings: AppSettings) -> Result<Self> {
        let qdrant_client = Qdrant::from_url(&settings.qdrant_url).build()?;
        let genai_client = Client::default();
        let model = settings.llm_model;

        // 1. Create the CUDA execution provider using the correct builder pattern from the documentation.
        let cuda_provider = CUDAExecutionProvider::default().build();

        // 2. Build the InitOptions, passing the provider in a Vec.
        let model_options = InitOptions::new(EmbeddingModel::AllMiniLML6V2)
            .with_execution_providers(vec![cuda_provider])
            .with_show_download_progress(true);

        // 3. Initialize the TextEmbedding model with the configured options.
        let embedding_model = Arc::new(
            TextEmbedding::try_new(model_options)
                .context("Failed to initialize embedding model with CUDA")?,
        );
        // initialize qdrant collection if !exists
        qdrant::ensure_collections_exist(&qdrant_client).await?;
        let http_client = Arc::new(reqwest::Client::new());
        Ok(Self {
            qdrant_client,
            genai_client,
            model,
            embedding_model: embedding_model,
            http_client,
        })
    }
}

// /// The core query processing logic.
// pub async fn process_query(query: &str, state: &AppState) -> Result<String> {
//     // Step 1: Search the web for fresh, real-time context.
//     let web_context = search_and_scrape(&state.http_client, query).await?;

//     // Step 2: Search our internal Qdrant databases for user-ingested knowledge
//     // and previously approved solutions.
//     let db_context = qdrant::search_for_context(state, query).await?;

//     // Step 3: Combine both contexts into a single, rich string.
//     // This gives the LLM the best of both worlds: live data and curated examples.
//     let combined_context = format!(
//         "Live Web Context:\n{}\n\nInternal Knowledge:\n{}",
//         web_context, db_context
//     );

//     // Step 4: Call the LLM with the augmented context to generate the code.
//     let generated_code = llm::generate_code(state, query, &combined_context).await?;

//     if generated_code.is_empty() {
//         return Ok("LLM failed to return a valid code block.".to_string());
//     }

//     // Step 5: Validate the AI-generated code in our secure sandbox.
//     let sandbox_result = sandbox::run_in_sandbox(&generated_code).await?;

//     // Step 6: Format the final response based on the success or failure of the sandbox compilation.
//     let response = if sandbox_result.success {
//         format!(
//             "OK. AI-generated code compiled successfully.\n---\n{}",
//             generated_code
//         )
//     } else {
//         format!(
//             "FAIL. AI-generated code failed to compile.\n---\nErrors:\n{}",
//             sandbox_result.output
//         )
//     };

//     Ok(response)
// }



/// The core query processing logic using a two-pass strategy.
pub async fn process_query(query: &str, state: &AppState) -> Result<String> {
    // === Step 1: Initial Context Gathering ===
    let web_context = search_and_scrape(&state.http_client, query).await?;
    let db_context = qdrant::search_for_context(state, query).await?;
    let initial_context = format!(
        "Live Web Context:\n{}\n\nInternal Knowledge:\n{}",
        web_context, db_context
    );

    // === Step 2: First Pass - Identify Required Crates ===
    let required_crates = llm::identify_required_crates(state, query, &initial_context).await?;
    println!("LLM identified required crates: {:?}", required_crates);

    // === Step 3: Research Step - Look Up Latest Crate Info ===
    let mut crate_research = String::new();
    for crate_name in &required_crates {
        let search_query = format!("crates.io rust crate {} latest API examples", crate_name);
        println!("Researching crate: {}", search_query);
        // Use your existing web_search module to perform the research!
        let search_results = web_search::search_and_scrape(&state.http_client, &search_query).await?;
        crate_research.push_str(&format!(
            "\n--- Research for crate '{}': ---\n{}\n",
            crate_name, search_results
        ));
    }

    // === Step 4: Second Pass - Generate Code with Up-to-Date Context ===
    let llm_response = llm::generate_code_with_research(state, query, &initial_context, &crate_research)
        .await
        .context("LLM failed to generate code in the second pass")?;

    if llm_response.code.is_empty() {
        return Ok("LLM failed to return a valid code block.".to_string());
    }

    // === Step 5: Sandbox Execution (No changes needed here) ===
    let sandbox_result =
        sandbox::run_in_sandbox(&llm_response.code, &llm_response.dependencies).await?;

    let response = if sandbox_result.success {
        format!(
            "OK. AI-generated code compiled successfully.\n---\n{}",
            llm_response.code
        )
    } else {
        format!(
            "FAIL. AI-generated code failed to compile.\n---\nErrors:\n{}",
            sandbox_result.output
        )
    };

    Ok(response)
}