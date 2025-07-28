use anyhow::Result;
use qdrant_client::{qdrant::{PointStruct, UpsertPoints}, Payload};
use crate::{qdrant::APPROVED_SOLUTIONS_COLLECTION, AppState};

/// Stores an upvoted solution in the Qdrant database
pub async fn process_upvoted_solution(
    state: &AppState,
    query: String,
    code: String,
) -> Result<()> {
    // Create a single embedding for the query-code pair to capture the semantic relationship.
    let text_to_embed = format!("Query: {}\n---\nCode:\n{}", query, code);
    let embedding = state.embedding_model.embed(vec![text_to_embed], None)?[0].clone();

    let payload:Payload = serde_json::json!({
        "query": query,
        "code": code,
    })
    .try_into()?;

    let point = PointStruct::new(uuid::Uuid::new_v4().to_string(), embedding, payload);

    state
        .qdrant_client
        .upsert_points(UpsertPoints {
            collection_name: APPROVED_SOLUTIONS_COLLECTION.to_string(),
            points: vec![point],
            ..Default::default()
        })
        .await?;

    Ok(())
}