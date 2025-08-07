// // In app_core/src/sandbox.rs

// use anyhow::{Context, Result};
// use std::collections::HashSet;
// use std::process::Output;
// use syn::visit::Visit;
// use syn::{File, ItemUse};
// use tempfile::TempDir;
// use tokio::fs;
// use tokio::process::Command;

// /// A simple AST visitor that only inspects `use` statements
// /// to find the root crate name.
// #[derive(Default)]
// struct CrateVisitor {
//     dependencies: HashSet<String>,
// }

// impl<'ast> Visit<'ast> for CrateVisitor {
//     fn visit_item_use(&mut self, i: &'ast ItemUse) {
//         // This helper function recursively finds the very first segment of a `use` path.
//         fn find_root_crate(tree: &syn::UseTree) -> Option<String> {
//             match tree {
//                 syn::UseTree::Path(path) => find_root_crate(&path.tree),
//                 syn::UseTree::Name(name) => Some(name.ident.to_string()),
//                 syn::UseTree::Rename(rename) => Some(rename.ident.to_string()),
//                 _ => None,
//             }
//         }

//         if let Some(ident) = find_root_crate(&i.tree) {
//             // Filter out standard library crates immediately.
//             if ident != "std" && ident != "core" && ident != "alloc" {
//                 self.dependencies.insert(ident);
//             }
//         }
//     }
// }

// /// Parses the given Rust code to find all external crate dependencies from `use` statements.
// fn find_dependencies(code: &str) -> Result<HashSet<String>> {
//     let ast: File = syn::parse_file(code).context("Failed to parse Rust code into syntax tree")?;

//     let mut visitor = CrateVisitor::default();
//     visitor.visit_file(&ast);

//     Ok(visitor.dependencies)
// }

// /// Represents the result of a sandbox compilation check.
// pub struct SandboxResult {
//     pub success: bool,
//     pub output: String,
// }

// /// Creates a temporary Cargo project, adds dependencies found in `use` statements,
// /// injects the code, and runs `cargo build` to validate it.
// pub async fn run_in_sandbox(code: &str) -> Result<SandboxResult> {
//     let dependencies = find_dependencies(code)?;

//     let mut cargo_toml = String::from(
//         r#"[package]
// name = "sandbox"
// version = "0.1.0"
// edition = "2024"

// [dependencies]
// "#,
//     );
//     for dep in dependencies {
//         let dep_name_for_cargo = dep.replace('_', "-");
//         cargo_toml.push_str(&format!("\"{}\" = \"*\"\n", dep_name_for_cargo));
//     }

//     let temp_dir = TempDir::new().context("Failed to create temp directory")?;
//     let src_dir = temp_dir.path().join("src");
//     fs::create_dir_all(&src_dir).await.context("Failed to create src directory")?;

//     fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)
//         .await
//         .context("Failed to write dynamic Cargo.toml")?;
//     fs::write(src_dir.join("main.rs"), code)
//         .await
//         .context("Failed to write main.rs")?;

//     let build_output = Command::new("cargo")
//         .arg("build")
//         .current_dir(temp_dir.path())
//         .output()
//         .await
//         .context("Failed to execute cargo build")?;

//     let result_output = if !build_output.status.success() {
//         String::from_utf8(build_output.stderr).context("Failed to read stderr from cargo build")?
//     } else {
//         String::new()
//     };

//     Ok(SandboxResult {
//         success: build_output.status.success(),
//         output: result_output,
//     })
// }

// In app_core/src/sandbox.rs

use anyhow::{Context, Result};
use std::process::Output;
use tempfile::TempDir;
use tokio::fs;
use tokio::process::Command;

// We need a struct to pass the dependency info to the sandbox.
// It's good practice to define this where it's used or in a shared module.
pub use crate::llm::Dependency;

/// Represents the result of a sandbox compilation check.
pub struct SandboxResult {
    pub success: bool,
    pub output: String,
}

/// Creates a temporary Cargo project with explicit dependencies and features.
pub async fn run_in_sandbox(code: &str, dependencies: &[Dependency]) -> Result<SandboxResult> {
    let mut cargo_toml = String::from(
        r#"[package]
name = "sandbox"
version = "0.1.0"
edition = "2024"

[dependencies]
"#,
    );

    for dep in dependencies {
        let features_str = dep.features.iter()
            .map(|f| format!("\"{}\"", f))
            .collect::<Vec<_>>()
            .join(", ");
        
        // Correctly format the dependency line with features
        cargo_toml.push_str(&format!(
            "{} = {{ version = \"*\", features = [{}] }}\n",
            dep.name, features_str
        ));
    }

    let temp_dir = TempDir::new().context("Failed to create temp directory")?;
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).await.context("Failed to create src directory")?;

    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)
        .await
        .context("Failed to write dynamic Cargo.toml")?;
    fs::write(src_dir.join("main.rs"), code)
        .await
        .context("Failed to write main.rs")?;

    let build_output = Command::new("cargo")
        .arg("build")
        .current_dir(temp_dir.path())
        .output()
        .await
        .context("Failed to execute cargo build")?;

    let result_output = if !build_output.status.success() {
        String::from_utf8(build_output.stderr).context("Failed to read stderr from cargo build")?
    } else {
        String::new()
    };

    Ok(SandboxResult {
        success: build_output.status.success(),
        output: result_output,
    })
}