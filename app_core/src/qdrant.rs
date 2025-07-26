use anyhow::Result;
use qdrant_client::{
    qdrant::{vectors_config::Config, CreateCollection, Distance, SearchPoints, VectorParams, VectorsConfig},
    Qdrant,
};

use crate::AppState;

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

/// Searches the knowledge base for relevant context.
pub async fn search_knowledge_base(state: &AppState, query: &str) -> Result<String> {
    let query_embedding = state.embedding_model.embed(vec![query.to_string()], None)?[0].clone();

    let search_response = state
        .qdrant_client
        .search_points(SearchPoints {
            collection_name: KNOWLEDGE_BASE_COLLECTION.to_string(),
            vector: query_embedding,
            limit: 3, // Find the top 3 most similar chunks
            with_payload: Some(true.into()),
            ..Default::default()
        })
        .await?;

    let context = search_response
        .result
        .into_iter()
        .filter_map(|point| point.payload.get("chunk")?.as_str().map(String::from))
        .collect::<Vec<String>>()
        .join("\n---\n");

    Ok(context)
}
