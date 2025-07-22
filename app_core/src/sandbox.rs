use anyhow::{Context, Result};
use std::process::Output;
use tempfile::TempDir;
use tokio::fs;
use tokio::process::Command;

const MINIMAL_CARGO_TOML: &str = r#"
[package]
name = "sandbox"
version = "0.1.0"
edition = "2024"

[dependencies]
"#;

/// Represents the result of a sandbox compilation check.
pub struct SandboxResult {
    pub success: bool,
    pub output: String,
}

/// Creates a temporary Cargo project, injects the provided code,
/// and runs `cargo check` to validate it.
pub async fn run_in_sandbox(code: &str) -> Result<SandboxResult> {
    let temp_dir = TempDir::new().context("Failed to create temp directory")?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)
        .await
        .context("Failed to create src directory")?;

    fs::write(temp_dir.path().join("Cargo.toml"), MINIMAL_CARGO_TOML)
        .await
        .context("Failed to write Cargo.toml")?;

    fs::write(src_dir.join("main.rs"), code)
        .await
        .context("Failed to write main.rs")?;

    let output: Output = Command::new("cargo")
        .arg("check")
        .current_dir(temp_dir.path())
        .output()
        .await
        .context("Failed to execute cargo check")?;

    let result_output = if output.status.success() {
        String::from_utf8(output.stdout).context("Failed to read stdout")?
    } else {
        String::from_utf8(output.stderr).context("Failed to read stderr")?
    };

    Ok(SandboxResult {
        success: output.status.success(),
        output: result_output,
    })
}
