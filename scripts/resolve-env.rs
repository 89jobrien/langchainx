#!/usr/bin/env rust-script
//! Resolve langchainx environment variables from 1Password and print them
//! as `KEY=value` lines suitable for `source`ing in nushell.
//!
//! Usage (in .nuenv):
//!   rust-script scripts/resolve-env.rs | lines | parse "{k}={v}" | each { $env.($in.k) = $in.v }
//!
//! ```cargo
//! [dependencies]
//! which = "6"
//! ```

use std::process::Command;

enum Source<'a> {
    Plugin { plugin: &'a str, env_var: &'a str },
    Op(&'a str),
    Default(&'a str),
}

struct Var<'a> {
    name: &'a str,
    source: Source<'a>,
}

fn op_available() -> bool {
    which::which("op").is_ok()
}

fn try_op(ref_path: &str) -> Option<String> {
    if !op_available() { return None; }
    let out = Command::new("op").args(["read", ref_path]).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn try_op_plugin(plugin: &str, env_var: &str) -> Option<String> {
    if !op_available() { return None; }
    let out = Command::new("op")
        .args(["plugin", "run", "--", plugin, "env"])
        .output()
        .ok()?;
    if !out.status.success() { return None; }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            if k == env_var { Some(v.to_string()) } else { None }
        })
}

fn main() {
    let vars: &[Var] = &[
        // ── LLM backends ─────────────────────────────────────────────────────
        Var { name: "OPENAI_API_KEY",    source: Source::Plugin { plugin: "openai",  env_var: "OPENAI_API_KEY" } },
        Var { name: "ANTHROPIC_API_KEY", source: Source::Op("op://Personal/Anthropic API Key/credential") },
        Var { name: "DEEPSEEK_API_KEY",  source: Source::Op("op://Personal/DeepSeek/credential") },
        Var { name: "QWEN_API_KEY",      source: Source::Op("op://Personal/Qwen/credential") },
        Var { name: "MISTRAL_API_KEY",   source: Source::Op("op://Personal/MistralAI/credential") },
        Var { name: "OLLAMA_HOST",       source: Source::Default("http://localhost:11434") },
        // ── Vector stores ─────────────────────────────────────────────────────
        Var { name: "POSTGRES_URL",      source: Source::Default("postgresql://localhost:5432/langchainx") },
        Var { name: "SURREALDB_URL",     source: Source::Default("ws://localhost:8000") },
        Var { name: "SURREALDB_NS",      source: Source::Default("langchainx") },
        Var { name: "SURREALDB_DB",      source: Source::Default("langchainx") },
        Var { name: "SURREALDB_USER",    source: Source::Op("op://Personal/SurrealDB/username") },
        Var { name: "SURREALDB_PASS",    source: Source::Op("op://Personal/SurrealDB/password") },
        Var { name: "QDRANT_URL",        source: Source::Default("http://localhost:6334") },
        Var { name: "QDRANT_API_KEY",    source: Source::Op("op://Personal/Qdrant/credential") },
        Var { name: "OPENSEARCH_URL",    source: Source::Default("http://localhost:9200") },
        Var { name: "OPENSEARCH_USER",   source: Source::Default("") },
        Var { name: "OPENSEARCH_PASS",   source: Source::Op("op://Personal/OpenSearch/password") },
        Var { name: "SQLITE_PATH",       source: Source::Default("./langchainx.db") },
        // ── Tools ─────────────────────────────────────────────────────────────
        Var { name: "SERPAPI_API_KEY",   source: Source::Op("op://Personal/SerpAPI/credential") },
        Var { name: "WOLFRAM_APP_ID",    source: Source::Op("op://Personal/Wolfram/credential") },
        // ── Dev / test ────────────────────────────────────────────────────────
        Var { name: "RUST_LOG",          source: Source::Default("langchainx=debug") },
    ];

    for var in vars {
        // skip if already set in environment
        if std::env::var(var.name).is_ok() { continue; }

        let value = match &var.source {
            Source::Plugin { plugin, env_var } => {
                try_op_plugin(plugin, env_var).unwrap_or_default()
            }
            Source::Op(r) => try_op(r).unwrap_or_default(),
            Source::Default(v) => v.to_string(),
        };

        println!("{}={}", var.name, value);
    }
}
