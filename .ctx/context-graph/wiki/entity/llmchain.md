---
title: LLMChain
type: struct
tags: [entity, struct]
---

# LLMChain

**Type:** struct

- basic prompt->LLM pipeline

## Outgoing Relations
- --[implements]--> [[chain]] (conf: 1.0)
- --[depends_on]--> [[llm]] (conf: 1.0)
- --[depends_on]--> [[formatprompter]] (conf: 1.0)
- --[depends_on]--> [[outputparser]] (conf: 1.0)

## Incoming Relations
- [[llmchainbuilder]] --[related_to]--> this (conf: 1.0)
