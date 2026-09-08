---
title: Workspace Architecture
source_document: arch_workspace
tags: [summary, architecture]
---

# Workspace Architecture

langchainx is a Rust port of LangChain organized as a Cargo workspace.

## Crate Dependency Graph

```
langchainx (facade)
  +-- langchainx-core (schemas, errors, Tool trait)
  +-- langchainx-llm (LLM trait, backends)
  |     +-- langchainx-core
  +-- langchainx-prompt (templates, formatting macros)
  +-- langchainx-chain (Chain trait, LLMChain, builders)
  |     +-- langchainx-core, langchainx-llm
  +-- langchainx-agent (AgentExecutor, ChatAgent)
  |     +-- langchainx-core, langchainx-chain
  +-- langchainx-memory (SimpleMemory, WindowBuffer)
  +-- langchainx-embedding (Embedder trait, backends)
  +-- langchainx-output-parsers
  +-- langchainx-tools (built-in tools)
  +-- langchainx-loaders (document loaders)
  +-- langchainx-vectorstore (VectorStore trait, backends)
  +-- langchainx-text-splitter
  +-- langchainx-macros (tool!, llm!, prompt!, chain!)
        +-- langchainx-core, langchainx-prompt, langchainx-chain
```

## Core Traits

- [[LLM-trait]] — `generate`, `invoke`, `stream`
- [[Chain-trait]] — `call`, `invoke`, `execute`, `stream`
- [[Tool-trait]] — `name`, `description`, `parameters`, `run`
- [[Embedder-trait]] — embedding vectors
- [[VectorStore-trait]] — `add_documents`, `similarity_search`

## Pattern

All traits use `async-trait`. Backends are feature-gated.
The facade crate re-exports everything for convenience.
