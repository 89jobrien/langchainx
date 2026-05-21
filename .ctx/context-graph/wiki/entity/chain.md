---
title: Chain
type: trait
tags: [entity, trait, core]
---

# Chain

Core async trait for processing pipelines. Methods: call(PromptArgs) -> GenerateResult, invoke(PromptArgs) -> String, execute(PromptArgs) -> HashMap, stream(PromptArgs) -> Stream. Implementors: [[llmchain]], [[conversationalchain]], [[conversationalretrievalqa]], [[sequentialchain]], [[stuffdocuments]], [[agentexecutor]].
