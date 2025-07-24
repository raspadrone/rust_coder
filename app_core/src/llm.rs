use anyhow::Result;
use genai::{chat::{ChatMessage, ChatRequest}, Client};
use anyhow::anyhow;

/// Extracts Rust code from a string that might contain markdown code fences.
fn extract_rust_code(content: &str) -> String {
    // Check if the code is wrapped in markdown fences
    if let Some(start) = content.find("```rust") {
        return content[start + 7..]
            .lines()
            .take_while(|line| !line.starts_with("```"))
            .collect::<Vec<_>>()
            .join("\n");
    }
    // Otherwise, assume the whole content is the code
    content.to_string()
}


/// Generates code using the genai crate.
pub async fn generate_code(genai_client: &Client, model: &str) -> Result<String> {
    let system_prompt = "You are a Rust programming assistant. Your response must only be a single, valid Rust code block enclosed in ```rust. Do not include any other explanations or text. The code should be a complete, runnable program.";

    let request = ChatRequest::new(vec![ChatMessage::user(system_prompt)]);
        let response = genai_client.exec_chat(model, request, None).await?;

    let generated_content = response.content_text_as_str()
        .ok_or_else(|| anyhow!("o text content found in LLM response"))?;

    Ok(extract_rust_code(&generated_content))
}