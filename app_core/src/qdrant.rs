use anyhow::Result;
use qdrant_client::{
    Qdrant,
    qdrant::{
        CreateCollection, Distance, SearchPoints, VectorParams, VectorsConfig,
        vectors_config::Config,
    },
};

use crate::AppState;

pub const KNOWLEDGE_BASE_COLLECTION: &str = "knowledge_base";
pub const APPROVED_SOLUTIONS_COLLECTION: &str = "approved_solutions";

/// Creates the primary collection for storing knowledge base vectors if it doesn't exist.
pub async fn ensure_collections_exist(client: &Qdrant) -> Result<()> {
    let collections_to_ensure = vec![KNOWLEDGE_BASE_COLLECTION, APPROVED_SOLUTIONS_COLLECTION];
    for collection_name in collections_to_ensure {
        if client.collection_info(collection_name).await.is_err() {
            client
                .create_collection(CreateCollection {
                    collection_name: collection_name.to_string(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(Config::Params(VectorParams {
                            size: 384, // AllMiniLML6V2 uses 384-dimensional embeddings
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        })),
                    }),
                    ..Default::default()
                })
                .await?;
            println!("INFO: Created Qdrant collection '{}'", collection_name);
        }
    }
    Ok(())
}

// /// Searches the knowledge base for relevant context.
// pub async fn search_knowledge_base(state: &AppState, query: &str) -> Result<String> {
//     let query_embedding = state.embedding_model.embed(vec![query.to_string()], None)?[0].clone();

//     let search_response = state
//         .qdrant_client
//         .search_points(SearchPoints {
//             collection_name: KNOWLEDGE_BASE_COLLECTION.to_string(),
//             vector: query_embedding,
//             limit: 3, // Find the top 3 most similar chunks
//             with_payload: Some(true.into()),
//             ..Default::default()
//         })
//         .await?;

//     let context = search_response
//         .result
//         .into_iter()
//         .filter_map(|point| point.payload.get("chunk")?.as_str().map(String::from))
//         .collect::<Vec<String>>()
//         .join("\n---\n");

//     Ok(context)
// }

/// Searches both the knowledge base and approved solutions for relevant context.
pub async fn search_for_context(state: &AppState, query: &str) -> Result<String> {
    let query_embedding = state.embedding_model.embed(vec![query.to_string()], None)?[0].clone();

    // Search the knowledge base for general documentation
    let knowledge_search = state.qdrant_client.search_points(SearchPoints {
        collection_name: KNOWLEDGE_BASE_COLLECTION.to_string(),
        vector: query_embedding.clone(),
        limit: 2,
        with_payload: Some(true.into()),
        ..Default::default()
    });

    // Search the approved solutions for golden examples
    let approved_search = state.qdrant_client.search_points(SearchPoints {
        collection_name: APPROVED_SOLUTIONS_COLLECTION.to_string(),
        vector: query_embedding,
        limit: 1,
        with_payload: Some(true.into()),
        ..Default::default()
    });

    // Run both searches concurrently
    let (knowledge_res, approved_res) = tokio::join!(knowledge_search, approved_search);

    let mut context_parts = Vec::new();

    if let Ok(res) = knowledge_res {
        let knowledge_context = res
            .result
            .into_iter()
            .filter_map(|point| point.payload.get("chunk")?.as_str().map(String::from))
            .collect::<Vec<String>>()
            .join("\n---\n");
        if !knowledge_context.is_empty() {
            context_parts.push(format!("Relevant Documentation:\n{}", knowledge_context));
        }
    }

    if let Ok(res) = approved_res {
        let approved_context = res
            .result
            .into_iter()
            .filter_map(|point| {
                let code = point.payload.get("code")?.as_str()?;
                let original_query = point.payload.get("query")?.as_str()?;
                Some(format!(
                    "Previously approved solution for a similar query ('{}'):\n```rust\n{}\n```",
                    original_query, code
                ))
            })
            .collect::<Vec<String>>()
            .join("\n---\n");
        if !approved_context.is_empty() {
            context_parts.push(format!("Golden Example:\n{}", approved_context));
        }
    }

    Ok(context_parts.join("\n\n"))
}
