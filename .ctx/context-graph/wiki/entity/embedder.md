---
title: Embedder
type: trait
tags: [entity, trait, core]
---

# Embedder

Core async trait for text embeddings. Methods: embed_documents(&[String]) -> Vec<Vec<f64>>, embed_query(&str) -> Vec<f64>. Implementors: [[openai-embedder]], [[ollama-embedder]], [[fastembed]], [[mistralai-embedder]].
