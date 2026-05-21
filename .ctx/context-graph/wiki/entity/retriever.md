---
title: Retriever
type: trait
tags: [entity, trait]
---

# Retriever

**Type:** trait

- async trait Retriever: get_relevant_documents

## Outgoing Relations
- --[depends_on]--> [[document]] (conf: 1.0)

## Incoming Relations
- [[conversationalretrievalqa]] --[depends_on]--> this (conf: 1.0)
- [[vectorstore-retriever]] --[implements]--> this (conf: 1.0)
