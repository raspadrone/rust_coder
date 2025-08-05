use anyhow::Result;
use anyhow::anyhow;
use genai::chat::{ChatMessage, ChatRequest};

use crate::AppState;

/// Extracts Rust code from a string that might contain markdown code fences.
fn extract_rust_code(content: &str) -> String {
    if let Some(start) = content.find("```rust") {
        return content[start + 7..]
            .lines()
            .take_while(|line| !line.starts_with("```"))
            .collect::<Vec<_>>()
            .join("\n");
    }
    content.to_string()
}

/// Generates code using the genai crate.
pub async fn generate_code(state: &AppState, query: &str, context: &str) -> Result<String> {
    let system_prompt = r#"You are an expert Rust programmer. Your task is to answer the user's query by providing a single, complete, and runnable Rust code block.

IMPORTANT RULES:
1.  You MUST use fully qualified names for all types, functions, and modules (e.g., `std::collections::HashMap`, `linfa_trees::DecisionTree`).
2.  Do NOT include complex `use` statements like `use linfa::prelude::*;` or `use linfa_trees::{DecisionTree};`. Only use simple `use` statements for the crate names themselves if absolutely necessary (e.g., `use linfa;`).
3.  The code must be self-contained within a `main` function.
4.  Do not wrap the code in markdown backticks ```rust ... ```. Only output the raw code.
"#;

    // Combine context and query into a single, rich user prompt
    let user_prompt = format!(
        "CONTEXT:\n{}\n\n---\n\nTASK: Based on the provided context, answer the following query:\n{}",
        context, query
    );

    let request = ChatRequest::new(vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(&user_prompt),
    ]);

    let response = &state
        .genai_client
        .exec_chat(&state.model, request, None)
        .await?;

    let generated_content = response
        .content_text_as_str()
        .ok_or_else(|| anyhow!("No text content found in LLM response"))?;

    Ok(extract_rust_code(&generated_content))
}
