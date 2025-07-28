use std::sync::Arc;

use anyhow::Result;
use config::{Config, File};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use genai::Client;
use qdrant_client::Qdrant;
use serde::Deserialize;

use crate::{llm::generate_code, sandbox::run_in_sandbox};

pub mod llm;
pub mod sandbox;
pub mod qdrant;
pub mod ingestion;
pub mod feedback;


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
}

impl AppState {
    /// Initializes the application state, connecting to required services.
    pub async fn new(settings: AppSettings) -> Result<Self> {
        let qdrant_client = Qdrant::from_url(&settings.qdrant_url).build()?;
        let genai_client = Client::default();
        let model = settings.llm_model;
        // Initialize the embedding model
        let embedding_model = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))?;
        // initialize qdrant collection if !exists
        qdrant::ensure_collection_exists(&qdrant_client).await?;
        Ok(Self {
            qdrant_client,
            genai_client,
            model,
            embedding_model: Arc::new(embedding_model)
        })
    }
}

/// The core query processing logic.
pub async fn process_query(query: &str, state: &AppState) -> Result<String> {
    // Search for relevant context in BOTH knowledge bases
    let context = qdrant::search_for_context(state, query).await?;

    // Generate code from the LLM, now with added context
    let generated_code = generate_code(state, query, &context).await?;
    // let generated_code = generate_code(&state, query).await?;

    if generated_code.is_empty() {
        return Ok("LLM failed to return a valid code block.".to_string());
    }
    let sandbox_result = run_in_sandbox(&generated_code).await?;

    let response = if sandbox_result.success {
        format!(
            "Sandbox test for query '{}' succeeded! Output:\n---\n{}",
            query, sandbox_result.output
        )
    } else {
        format!(
            "Sandbox test for query '{}' failed! Compiler errors:\n---\n{}",
            query, sandbox_result.output
        )
    };

    Ok(response)
}
