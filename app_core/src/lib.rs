use anyhow::Result;
use config::{Config, File};
use qdrant_client::Qdrant;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct AppSettings {
    pub qdrant_url: String,
}

#[derive(Clone)]
pub struct AppState {
    pub qdrant_client: Qdrant,
}

impl AppState {
    /// Initializes the application state, connecting to required services.
    pub async fn new(settings: AppSettings) -> Result<Self> {
        let qdrant_client = Qdrant::from_url(&settings.qdrant_url).build()?;
        Ok(Self { qdrant_client })
    }
}

/// The core query processing logic.
pub async fn process_query(query: &str, state: &AppState) -> Result<String> {
    // For now, just prove we can connect to Qdrant
    let collections_list = state.qdrant_client.list_collections().await?;
    let response = format!(
        "Query: '{}'. Found {} collections in Qdrant.",
        query,
        collections_list.collections.len()
    );
    Ok(response)
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