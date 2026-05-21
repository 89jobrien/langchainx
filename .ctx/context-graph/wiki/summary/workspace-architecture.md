---
title: Workspace Architecture
source_document: arch_workspace
tags: [summary, architecture]
---

# Workspace Architecture

langchainx is a Rust port of LangChain organized as a Cargo workspace (edition 2024, v0.4.6).

## Crates

| Crate | Purpose |
|-------|---------|
| [[langchainx]] | Facade: re-exports from all sub-crates |
| [[langchainx-core]] | Shared types, traits, errors |
| [[langchainx-llm]] | LLM backends (OpenAI, Claude, DeepSeek, Qwen, Ollama) |
| [[langchainx-chain]] | Chain abstractions (LLMChain, Conversational, Sequential) |
| [[langchainx-prompt]] | Prompt templates and macros |
| [[langchainx-memory]] | Memory implementations |
| [[langchainx-embedding]] | Embedding backends |
| [[langchainx-vectorstore]] | Vector store backends |
| [[langchainx-agent]] | Agent executor and agent types |
| [[langchainx-tools]] | Tool trait and implementations |
| [[langchainx-loaders]] | Document loaders |
| [[langchainx-text-splitter]] | Text splitting |
| [[langchainx-semantic-router]] | Semantic routing |
| [[langchainx-output-parsers]] | Output parsing |

## Feature Flags

Optional integrations gated behind Cargo features: postgres, qdrant, surrealdb,
opensearch, sqlite-vss, sqlite-vec, ollama, fastembed, mistralai, git, lopdf,
pdf-extract, html-to-markdown, tree-sitter, rss, sitemap.
