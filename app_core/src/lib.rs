use anyhow::Result;
use config::{Config, File};
use genai::Client;
use qdrant_client::Qdrant;
use serde::Deserialize;

use crate::{llm::generate_code, sandbox::run_in_sandbox};

pub mod llm;
pub mod sandbox;
pub mod qdrant;


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
}

impl AppState {
    /// Initializes the application state, connecting to required services.
    pub async fn new(settings: AppSettings) -> Result<Self> {
        let qdrant_client = Qdrant::from_url(&settings.qdrant_url).build()?;
        let genai_client = Client::default();
        let model = settings.llm_model;
        Ok(Self {
            qdrant_client,
            genai_client,
            model,
        })
    }
}

/// The core query processing logic.
pub async fn process_query(query: &str, state: &AppState) -> Result<String> {
    let generated_code = generate_code(&state, query).await?;

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
