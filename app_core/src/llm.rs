use anyhow::Result;
use genai::chat::{ChatMessage, ChatRequest};
use anyhow::anyhow;

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
    let system_prompt = "You are a Rust programming assistant. Your response must only be a single, valid Rust code block enclosed in ```rust. Do not include any other explanations or text. The code should be a complete, runnable program.";

    // Combine context and query into a single, rich user prompt
    let user_prompt = format!(
        "CONTEXT:\n{}\n\n---\n\nTASK: Based on the provided context, answer the following query:\n{}",
        context, query
    );

    let request = ChatRequest::new(vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(&user_prompt)
    ]);

    let response = &state.genai_client.exec_chat(&state.model, request, None).await?;

    let generated_content = response.content_text_as_str()
        .ok_or_else(|| anyhow!("No text content found in LLM response"))?;

    Ok(extract_rust_code(&generated_content))
}