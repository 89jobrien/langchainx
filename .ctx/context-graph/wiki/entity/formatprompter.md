---
title: FormatPrompter
type: trait
tags: [entity, trait]
---

# FormatPrompter

**Type:** trait

- trait FormatPrompter: format_prompt -> PromptValue

## Outgoing Relations
- --[depends_on]--> [[promptvalue]] (conf: 1.0)
- --[depends_on]--> [[promptargs]] (conf: 1.0)

## Incoming Relations
- [[prompttemplate]] --[implements]--> this (conf: 1.0)
- [[llmchain]] --[depends_on]--> this (conf: 1.0)
- [[messageformatter]] --[implements]--> this (conf: 1.0)
