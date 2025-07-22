use anyhow::Result;
use config::{Config, File};
use qdrant_client::Qdrant;
use serde::Deserialize;

use crate::sandbox::run_in_sandbox;

pub mod sandbox;

#[derive(Deserialize, Clone)]
pub struct AppSettings {
    pub qdrant_url: String,
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
}

impl AppState {
    /// Initializes the application state, connecting to required services.
    pub async fn new(settings: AppSettings) -> Result<Self> {
        let qdrant_client = Qdrant::from_url(&settings.qdrant_url).build()?;
        Ok(Self { qdrant_client })
    }
}

/// The core query processing logic.
pub async fn process_query(query: &str, _state: &AppState) -> Result<String> {
    // For now, we'll test the sandbox with a hardcoded, valid piece of code.
    let code_to_test = r#"
        // fn main() {
        //     println!("This code compiles!");
        // }
        fn main( { let x = ; }
    "#;

    let sandbox_result = run_in_sandbox(code_to_test).await?;

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
