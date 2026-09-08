---
title: PromptTemplate
type: struct
tags: [entity, struct]
---

# PromptTemplate

**Type:** struct

- template string with variables, FString or Jinja2 format

## Outgoing Relations
- --[implements]--> [[formatprompter]] (conf: 1.0)

## Incoming Relations
- [[template-fstring-macro]] --[related_to]--> this (conf: 1.0)
- [[humanmessageprompttemplate]] --[depends_on]--> this (conf: 1.0)
