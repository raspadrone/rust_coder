use anyhow::{anyhow, Context, Result};
use genai::chat::{ChatMessage, ChatRequest};
use serde::Deserialize;
use crate::AppState;

// This struct is for the FINAL response (code + deps with features)
#[derive(Deserialize, Debug)]
pub struct Dependency {
    pub name: String,
    pub features: Vec<String>,
}
#[derive(Deserialize, Debug)]
pub struct LlmCodeResponse {
    pub dependencies: Vec<Dependency>,
    pub code: String,
}

// This struct is for the FIRST planning response
#[derive(Deserialize, Debug)]
struct LlmCratePlan {
    crates: Vec<String>,
}

/// FIRST PASS: Identifies which crates are needed to answer a query.
pub async fn identify_required_crates(
    state: &AppState,
    query: &str,
    context: &str,
) -> Result<Vec<String>> {
    let system_prompt = r#"You are a Rust project planning expert. Your task is to analyze a user's query and the provided context, and determine which external crates from crates.io are necessary to solve the problem.

Respond with a single JSON object containing one key: `"crates"`, which should be an array of strings. The strings should be the real, kebab-case names of the required crates as they appear on crates.io.

EXAMPLE:
{
  "crates": ["linfa", "linfa-trees", "linfa-datasets"]
}"#;

    let user_prompt = format!(
        "CONTEXT:\n{}\n\n---\n\nTASK: Based on the context, identify the crates needed to answer this query:\n{}",
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
        .ok_or_else(|| anyhow!("No text content found in LLM planning response"))?;

    let json_start = generated_content.find('{').unwrap_or(0);
    let json_end = generated_content.rfind('}').unwrap_or_else(|| generated_content.len());
    let json_str = &generated_content[json_start..=json_end];

    let plan: LlmCratePlan = serde_json::from_str(json_str)
        .with_context(|| format!("Failed to parse crate plan from LLM: {}", json_str))?;

    Ok(plan.crates)
}

/// SECOND PASS: Generates code using the researched, up-to-date crate information.
pub async fn generate_code_with_research(
    state: &AppState,
    query: &str,
    context: &str,
    crate_research: &str,
) -> Result<LlmCodeResponse> {
    // This prompt now includes the critical, reinforced rule about importing traits.
    let system_prompt = format!(
        r#"You are an expert Rust programmer. You will be given context, a user query, and up-to-date research on real crates from crates.io. Your task is to provide a single, high-quality JSON object.

# UP-TO-DATE CRATE RESEARCH
{}

# RULES
1.  **CRITICAL**: You MUST include a `use` statement for any TRAITS that provide methods you are using. For example, to use the `.forward()` method in the `candle` crate, you MUST include `use candle_core::Module;`. This is the most important rule.
2.  The code you generate MUST be pure Rust and depend ONLY on real crates from crates.io as detailed in the research. It CANNOT require any external programs or libraries from other languages.
3.  You MUST write code that is compatible with the latest crate versions found in the research provided.
4.  The JSON object you provide MUST contain two keys:
    a. `"dependencies"`: An array of objects. Each object must have a `"name"` (string, kebab-case) and a `"features"` (array of strings) key.
    b. `"code"`: A string containing the complete, runnable Rust code, self-contained in a `main` function.
5.  When printing a struct or other complex type, you MUST use the debug formatter `{{:?}}`.
"#,
        crate_research
    );

    let user_prompt = format!(
        "CONTEXT:\n---\n{}\n---\n\nTASK: Based on all provided context and research, generate a JSON response that answers the following query.\n\nQUERY: {}",
        context, query
    );

    let request = ChatRequest::new(vec![
        ChatMessage::system(&system_prompt),
        ChatMessage::user(&user_prompt),
    ]);

    let response = &state
        .genai_client
        .exec_chat(&state.model, request, None)
        .await?;
        
    let generated_content = response
        .content_text_as_str()
        .ok_or_else(|| anyhow!("No text content found in LLM generation response"))?;

    let json_start = generated_content.find('{').unwrap_or(0);
    let json_end = generated_content.rfind('}').unwrap_or_else(|| generated_content.len());
    let json_str = &generated_content[json_start..=json_end];

    serde_json::from_str(json_str)
        .with_context(|| format!("Failed to parse JSON from LLM response: {}", json_str))
}