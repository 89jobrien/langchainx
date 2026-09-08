---
title: Document
type: struct
tags: [entity, struct]
---

# Document

**Type:** struct

- page_content, metadata HashMap, score

## Incoming Relations
- [[vectorstore]] --[depends_on]--> this (conf: 1.0)
- [[retriever]] --[depends_on]--> this (conf: 1.0)
- [[loader]] --[depends_on]--> this (conf: 1.0)
