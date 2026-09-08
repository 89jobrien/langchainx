---
title: LLM
type: trait
tags: [entity, trait, core]
---

# LLM

Core async trait for language model inference. Methods: generate(&[Message]) -> GenerateResult, invoke(&str) -> String, stream(&[Message]) -> Stream<StreamData>. Defined in langchainx-llm. Implementors: [[openai-backend]], [[claude-backend]], [[deepseek-backend]], [[qwen-backend]], [[ollama-backend]].
