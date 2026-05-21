---
title: AgentExecutor
type: struct
tags: [entity, struct]
---

# AgentExecutor

**Type:** struct

- runs Agent.plan() loop dispatching tools

## Outgoing Relations
- --[implements]--> [[chain]] (conf: 1.0)
- --[depends_on]--> [[agent]] (conf: 1.0)
- --[depends_on]--> [[tool]] (conf: 1.0)
