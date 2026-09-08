---
title: VectorStore
type: trait
tags: [entity, trait, core]
---

# VectorStore

Core async trait for vector storage. Methods: add_documents(&[Document], &Options), similarity_search(&str, limit, &Options). Implementors: [[postgres-vectorstore]], [[qdrant-vectorstore]], [[opensearch-vectorstore]], [[sqlite-vss-vectorstore]], [[sqlite-vec-vectorstore]], [[surrealdb-vectorstore]].
