---
title: LLM Backends
source_document: llm_backends
tags: [summary, llm, backends]
---

# LLM Backends

All implement [[llm]] trait. Located in [[langchainx-llm]].

- [[openai-backend]] -- async-openai crate, GPT models, function calling
- [[claude-backend]] -- custom HTTP client, Anthropic API
- [[deepseek-backend]] -- OpenAI-compatible, custom base URL
- [[qwen-backend]] -- OpenAI-compatible, custom base URL
- [[ollama-backend]] -- local inference via ollama-rs (feature-gated)

[[calloptions]] controls temperature, max_tokens, stop_words.
