---
title: GenerateResult
type: struct
tags: [entity, struct]
---

# GenerateResult

**Type:** struct

- generation output with token usage

## Outgoing Relations
- --[contains]--> [[tokenusage]] (conf: 1.0)

## Incoming Relations
- [[llm]] --[depends_on]--> this (conf: 1.0)
- [[chain]] --[depends_on]--> this (conf: 1.0)
