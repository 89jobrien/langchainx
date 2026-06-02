---
title: Macros DSL
source_document: macros_design
tags: [summary, macros, dsl]
---

# Macros DSL

The [[langchainx-macros]] crate provides four `macro_rules!` macros.

## Macros

| Macro             | Purpose          | Expansion                                                 |
| ----------------- | ---------------- | --------------------------------------------------------- |
| [[tool!-macro]]   | Tool trait impl  | unit struct + `impl Tool`                                 |
| [[llm!-macro]]    | LLM construction | `Backend::default().with_key(val)...`                     |
| [[prompt!-macro]] | Prompt template  | `HumanMessagePromptTemplate::new(template_fstring!(...))` |
| [[chain!-macro]]  | Chain wiring     | `LLMChainBuilder::new().prompt().llm().build()`           |

## Composability

Macros nest: `chain!(prompt!(...), llm!(...))`.

## Key Pattern

[[llm!-macro]] uses [[recursive-peeling]] — each `key = val` pair has its
own `@build` arm that calls `.with_key(val)` and recurses on remaining pairs.

## SOLID Alignment

- DIP: generates trait-based code, not concrete types
- OCP: llm! handles new backends without modification
- ISP: one macro per concern
- LSP: generated impls substitutable with hand-written ones
