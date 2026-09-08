---
title: Agent
type: trait
tags: [entity, trait, core]
---

# Agent

Core async trait for agent planning. Methods: plan(steps, inputs) -> AgentEvent, get_tools() -> Vec<Arc<dyn Tool>>. Implementors: [[chatagent]], [[openaitoolsagent]].
