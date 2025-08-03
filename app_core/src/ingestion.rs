

use crate::{AppState, qdrant::KNOWLEDGE_BASE_COLLECTION};
use anyhow::{Context, Result};
use qdrant_client::Payload;
use qdrant_client::qdrant::{PointStruct, UpsertPoints};



use serde_json::json;
use text_splitter::{Characters, ChunkConfig, TextSplitter};



pub async fn ingest_document(state: AppState, document: String) -> Result<()> {
    let chunk_config = ChunkConfig::<Characters>::new(1000)
        .with_overlap(100)?
        .with_trim(true);

    let splitter = TextSplitter::new(chunk_config);
    let chunks: Vec<String> = splitter
        .chunks(&document)
        .map(|s| s.to_owned())
        .collect();
    println!("Document split into {} chunks. Processing in batches...", chunks.len());

    const BATCH_SIZE: usize = 32;

    for chunk_batch in chunks.chunks(BATCH_SIZE) {
        let batch_size = chunk_batch.len();
        println!("Processing batch of {} chunks...", batch_size);

        let model_arc = state.embedding_model.clone();
        let batch_to_embed = chunk_batch.to_vec();
        let embeddings = tokio::task::spawn_blocking(move || {
            model_arc.embed(batch_to_embed, None)
        })
        .await
        .context("Task panicked while generating embeddings")??;

        if embeddings.is_empty() {
            println!("Warning: Embedding generation returned no vectors for a batch.");
            continue;
        }

        let points: Vec<PointStruct> = embeddings
            .into_iter()
            .zip(chunk_batch.iter())
            .map(|(embedding, chunk_text)| {
                let payload: Payload = json!({ "text": *chunk_text }).try_into().unwrap();
                PointStruct::new(uuid::Uuid::new_v4().to_string(), embedding, payload)
            })
            .collect();

        // ======================== THE IMPORTANT CHANGE IS HERE ========================
        println!("Attempting to upsert batch of {} points...", points.len());
        
        // Capture the result instead of immediately using '?'
        let upsert_result = state
            .qdrant_client
            .upsert_points(UpsertPoints {
                collection_name: KNOWLEDGE_BASE_COLLECTION.to_string(),
                points,
                ..Default::default()
            })
            .await;

        // Print the full result from the Qdrant client
        println!("QDRANT RESPONSE: {:?}", upsert_result);
        
        // Now, handle the error if it exists
        upsert_result?;
        // ==============================================================================
    }

    println!("--- Ingestion Complete! All batches processed. ---");
    Ok(())
}