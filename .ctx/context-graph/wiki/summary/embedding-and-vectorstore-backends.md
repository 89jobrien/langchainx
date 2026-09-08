---
title: Embedding and VectorStore Backends
source_document: embed_vectorstore
tags: [summary, embedding, vectorstore]
---

# Embedding Backends

All implement [[embedder]]. In [[langchainx-embedding]].

- [[openai-embedder]] -- text-embedding models
- [[ollama-embedder]] -- local (feature-gated)
- [[fastembed]] -- local fastembed crate (feature-gated)
- [[mistralai-embedder]] -- Mistral API (feature-gated)

# VectorStore Backends

All implement [[vectorstore]]. In [[langchainx-vectorstore]].

- [[postgres-vectorstore]] -- pgvector
- [[qdrant-vectorstore]] -- Qdrant
- [[opensearch-vectorstore]] -- OpenSearch + AWS auth
- [[sqlite-vss-vectorstore]] -- SQLite VSS
- [[sqlite-vec-vectorstore]] -- SQLite vec
- [[surrealdb-vectorstore]] -- SurrealDB

[[vectorstore-retriever]] bridges VectorStore to [[retriever]] trait.
