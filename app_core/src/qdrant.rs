use anyhow::Result;
use qdrant_client::{
    qdrant::{vectors_config::Config, CreateCollection, Distance, VectorParams, VectorsConfig},
    Qdrant,
};

pub const KNOWLEDGE_BASE_COLLECTION: &str = "knowledge_base";

/// Creates the primary collection for storing knowledge base vectors if it doesn't exist.
pub async fn ensure_collection_exists(client: &Qdrant) -> Result<()> {
    let collections = client.list_collections().await?;
    let collection_exists = collections
        .collections
        .into_iter()
        .any(|c| c.name == KNOWLEDGE_BASE_COLLECTION);

    if !collection_exists {
        client
            .create_collection(CreateCollection {
                collection_name: KNOWLEDGE_BASE_COLLECTION.to_string(),
                vectors_config: Some(VectorsConfig {
                    config: Some(Config::Params(VectorParams {
                        size: 768, // Gemini models use 768-dimensional embeddings
                        distance: Distance::Cosine.into(),
                        ..Default::default()
                    })),
                }),
                ..Default::default()
            })
            .await?;
    }
    Ok(())
}