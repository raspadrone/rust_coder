// In app_core/src/sandbox.rs

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::process::Output;
use syn::visit::{self, Visit};
use syn::{File, Item, Path};
use tempfile::TempDir;
use tokio::fs;
use tokio::process::Command;

/// A visitor that walks the Rust AST and collects all potential crate dependencies
/// from both `use` statements and fully qualified paths.
#[derive(Default)]
struct CrateVisitor {
    dependencies: HashSet<String>,
}

impl<'ast> Visit<'ast> for CrateVisitor {
    /// Captures the root crate from `use` statements (e.g., `linfa` from `use linfa::prelude::*`).
    fn visit_item_use(&mut self, i: &'ast syn::ItemUse) {
        // This helper function recursively finds the very first segment of a `use` path.
        fn find_root_crate(tree: &syn::UseTree) -> Option<String> {
            match tree {
                syn::UseTree::Path(path) => find_root_crate(&path.tree),
                syn::UseTree::Name(name) => Some(name.ident.to_string()),
                syn::UseTree::Rename(rename) => Some(rename.ident.to_string()),
                _ => None,
            }
        }

        if let Some(root_crate) = find_root_crate(&i.tree) {
            self.dependencies.insert(root_crate);
        }
    }

    /// Captures the root crate from fully qualified paths (e.g., `linfa_trees` from `linfa_trees::DecisionTree`).
    fn visit_path(&mut self, path: &'ast Path) {
        if path.leading_colon.is_none() {
            if let Some(segment) = path.segments.iter().next() {
                // This captures the first part of any path, like `linfa_trees`
                self.dependencies.insert(segment.ident.to_string());
            }
        }
        // Continue visiting to check for nested paths.
        visit::visit_path(self, path);
    }
}

/// Parses the given Rust code to find all external crate dependencies.
fn find_dependencies(code: &str) -> Result<HashSet<String>> {
    let ast: File = syn::parse_file(code).context("Failed to parse Rust code into syntax tree")?;

    let mut visitor = CrateVisitor::default();
    visitor.visit_file(&ast);

    // Filter out standard library crates, keywords, and common primitives to avoid errors.
    let blacklist = [
        "std", "core", "alloc", "super", "self", "crate", "anyhow", "Result", "Ok", "Err",
        "println", "String", "Vec", "Option", "HashMap", "HashSet", "Default", "main"
    ];
    visitor.dependencies.retain(|dep| !blacklist.contains(&dep.as_str()));

    Ok(visitor.dependencies)
}

/// Represents the result of a sandbox compilation check.
pub struct SandboxResult {
    pub success: bool,
    pub output: String,
}

/// Creates a temporary Cargo project, dynamically adds dependencies by parsing the code,
/// injects the code, and runs `cargo build` to validate it.
pub async fn run_in_sandbox(code: &str) -> Result<SandboxResult> {
    let dependencies = find_dependencies(code)?;

    let mut cargo_toml = String::from(
        r#"[package]
name = "sandbox"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );
    for dep in dependencies {
        let dep_name_for_cargo = dep.replace('_', "-");
        cargo_toml.push_str(&format!("\"{}\" = \"*\"\n", dep_name_for_cargo));
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

// use anyhow::{Context, Result};
// use regex::Regex;
// use std::collections::HashSet;
// use std::process::Output;
// use tempfile::TempDir;
// use tokio::fs;
// use tokio::process::Command;

// /// Represents the result of a sandbox compilation check.
// pub struct SandboxResult {
//     pub success: bool,
//     pub output: String,
// }

// /// Creates a temporary Cargo project, dynamically adds dependencies based on `use` statements
// /// and qualified paths, injects the provided code, and runs `cargo check` to validate it.
// pub async fn run_in_sandbox(code: &str) -> Result<SandboxResult> {
//     // --- Step 1: Parse the code for crate dependencies ---

//     // This regex is designed to find the root crate in `use` statements.
//     // e.g., in `use my_crate::module::{Item1, Item2};`, it will only capture `my_crate`.
//     let use_re = Regex::new(r"use\s+((?P<crate>[a-zA-Z0-9_]+)::)?")
//         .context("Failed to compile 'use' statement regex")?;

//     // This regex finds crates used in qualified paths, e.g., `other_crate::function()`
//     let path_re = Regex::new(r"\b([a-zA-Z0-9_]+)::")
//         .context("Failed to compile qualified path regex")?;

//     let mut dependencies: HashSet<String> = use_re
//         .captures_iter(code)
//         .filter_map(|cap| cap.name("crate").map(|m| m.as_str().to_string()))
//         .collect();

//     for cap in path_re.captures_iter(code) {
//         dependencies.insert(cap[1].to_string());
//     }

//     // --- Create a blacklist of common Rust keywords and std library items ---
//     let blacklist: HashSet<&str> = [
//         "std", "core", "alloc", "super", "self", "crate", "anyhow",
//         // Common primitive types that might be captured by the path_re
//         "String", "Vec", "Option", "Result", "HashMap", "HashSet",
//     ].iter().cloned().collect();

//     // Filter out any blacklisted items
//     dependencies.retain(|dep| !blacklist.contains(dep.as_str()));


//     // --- Step 2: Build the dynamic Cargo.toml content ---
//     let mut cargo_toml = String::from(
//         r#"[package]
// name = "sandbox"
// version = "0.1.0"
// edition = "2024"

// [dependencies]
// "#,
//     );

//     for dep_name_in_code in &dependencies {
//         let dep_name_for_cargo = dep_name_in_code.replace('_', "-");
//         cargo_toml.push_str(&format!("\"{}\" = \"*\"\n", dep_name_for_cargo));
//     }
    
//     // --- Step 3: Create and run the sandbox ---
//     let temp_dir = TempDir::new().context("Failed to create temp directory")?;
//     let src_dir = temp_dir.path().join("src");
//     fs::create_dir_all(&src_dir).await.context("Failed to create src directory")?;

//     fs::write(temp_dir.path().join("Cargo.toml"), &cargo_toml)
//         .await
//         .context("Failed to write dynamic Cargo.toml")?;
//     fs::write(src_dir.join("main.rs"), code)
//         .await
//         .context("Failed to write main.rs")?;

//     // Run `cargo build` to fetch dependencies and compile.
//     let build_output = Command::new("cargo")
//         .arg("build")
//         .current_dir(temp_dir.path())
//         .output()
//         .await
//         .context("Failed to execute cargo build")?;

//     let result_output = if !build_output.status.success() {
//         String::from_utf8(build_output.stderr)
//             .context("Failed to read stderr from cargo build")?
//     } else {
//         // If build succeeds, the "output" is a success message.
//         // A full run would capture stdout from `cargo run`, but `build` is sufficient here.
//         String::from_utf8(build_output.stdout)
//             .context("Failed to read stdout from cargo build")?
//     };

//     Ok(SandboxResult {
//         success: build_output.status.success(),
//         output: result_output,
//     })
// }