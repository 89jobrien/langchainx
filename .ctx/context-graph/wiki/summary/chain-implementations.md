---
title: Chain Implementations
source_document: chain_impls
tags: [summary, chains]
---

# Chain Implementations

All implement [[chain]] trait. Located in [[langchainx-chain]] and [[langchainx-agent]].

- [[llmchain]] -- basic prompt->LLM, uses [[llmchainbuilder]]
- [[conversationalchain]] -- LLM + [[basememory]]
- [[conversationalretrievalqa]] -- memory + [[retriever]] (RAG)
- [[sequentialchain]] -- chains in series
- [[stuffdocuments]] -- stuff docs into prompt
- [[sqldatabase]] -- SQL from natural language
- [[agentexecutor]] -- Agent.plan() loop with tool dispatch
