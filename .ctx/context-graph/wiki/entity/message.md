---
title: Message
type: struct
tags: [entity, struct]
---

# Message

**Type:** struct

- content, message_type, tool_calls, images

## Outgoing Relations
- --[contains]--> [[messagetype]] (conf: 1.0)

## Incoming Relations
- [[llm]] --[depends_on]--> this (conf: 1.0)
- [[basememory]] --[depends_on]--> this (conf: 1.0)
