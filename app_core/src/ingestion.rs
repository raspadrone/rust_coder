use crate::{AppState, qdrant::KNOWLEDGE_BASE_COLLECTION};
use anyhow::{Context, Result};
use qdrant_client::Payload;
use qdrant_client::qdrant::{PointStruct, UpsertPoints};

// /// Ingests a document into the Qdrant knowledge base using fastembed.
// pub async fn ingest_document(state: AppState, document: String) -> Result<()> {
//     // Clone the Arc, not the model itself. This is cheap.
//     let model_arc = state.embedding_model.clone();

//     // Clone the document so it can be moved into the closure but still be
//     // available for the payload later.
//     let doc_for_embedding = document.clone();

//     let embeddings = tokio::task::spawn_blocking(move || {
//         model_arc.embed(vec![doc_for_embedding], None)
//     })
//     .await
//     .context("Task panicked while generating embeddings")??;

//     let embedding = embeddings
//         .get(0)
//         .context("Embedding generation returned no vectors")?
//         .to_vec();

//     // Use the original `document` string for the payload.
//     let payload: Payload = serde_json::json!({ "chunk": document }).try_into()?;

//     let point = PointStruct::new(uuid::Uuid::new_v4().to_string(), embedding, payload);

//     state
//         .qdrant_client
//         .upsert_points(UpsertPoints {
//             collection_name: KNOWLEDGE_BASE_COLLECTION.to_string(),
//             points: vec![point],
//             ..Default::default()
//         })
//         .await?;

//     Ok(())
// }

/// Ingests a document into the Qdrant knowledge base using fastembed.
pub async fn ingest_document(state: AppState, document: String) -> Result<()> {
    let model_arc = state.embedding_model.clone();
    let doc_for_embedding = document.clone();

    let embeddings = tokio::task::spawn_blocking(move || {
        model_arc.embed(vec![doc_for_embedding], None)
    })
    .await
    .context("Task panicked while generating embeddings")??;

    let embedding = embeddings
        .get(0)
        .context("Embedding generation returned no vectors")?
        .to_vec();

    let payload: Payload = serde_json::json!({ "chunk": document }).try_into()?;
    let point = PointStruct::new(uuid::Uuid::new_v4().to_string(), embedding, payload);

    state
        .qdrant_client
        .upsert_points(UpsertPoints {
            collection_name: KNOWLEDGE_BASE_COLLECTION.to_string(),
            points: vec![point],
            ..Default::default()
        })
        .await?;

    Ok(())
}