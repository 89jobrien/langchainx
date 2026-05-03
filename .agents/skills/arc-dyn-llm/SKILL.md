---
name: arc-dyn-llm
description: JOB-251 — Why Arc<dyn LLM> over LLMClone, migration pattern, and correct LLM ownership model.
---

<oneliner>
The LLMClone supertrait is an anti-pattern. Prefer Arc<dyn LLM> for shared ownership —
Arc is Clone by definition, no extra trait machinery needed.
</oneliner>

<problem>
## Current Problem (JOB-251)

`LLMClone` exists solely because `dyn LLM` is not `Clone`:

```rust
// CURRENT — anti-pattern
pub trait LLMClone {
    fn clone_box(&self) -> Box<dyn LLM>;
}
pub trait LLM: Sync + Send + LLMClone { ... }

// Used only in two places:
let stuff_chain = StuffDocumentBuilder::new().llm(llm.clone_box());
let condense_chain = CondenseQuestionGeneratorChain::new(llm.clone_box());
```

`clone_box()` allocates a new `Box` on every call. Every backend must also `derive(Clone)`.
</problem>

<preferred-pattern>
## Preferred Pattern: Arc<dyn LLM>

```rust
use std::sync::Arc;
use langchain_rust::language_models::llm::LLM;

// Cheap clone — just an atomic ref count increment
let llm: Arc<dyn LLM> = Arc::new(Claude::new());
let llm2 = Arc::clone(&llm);  // no Box allocation, no clone_box()

// Both chains share the same LLM instance
let stuff_chain = StuffDocumentBuilder::new().llm(Arc::clone(&llm));
let condense_chain = CondenseQuestionGeneratorChain::new(Arc::clone(&llm));
```

</preferred-pattern>

<current-workaround>
## Current Workaround (until JOB-251 is resolved)

`clone_box()` still works today. If you must clone an LLM to pass to two builders:

```rust
// Acceptable today — will be removed when JOB-251 is implemented
let llm: Box<dyn LLM> = Box::new(OpenAI::default());
let stuff = StuffDocumentBuilder::new().llm(llm.clone_box()).build()?;
let condense = CondenseQuestionGeneratorChain::new(llm.clone_box());
```

Do NOT add new code that introduces new `clone_box()` call sites. Prefer Arc.
</current-workaround>

<migration>
## Migration Checklist (implementing JOB-251)

1. Remove `LLMClone` supertrait from `LLM` definition
2. Remove blanket `impl<T: LLM + Clone> LLMClone for T`
3. Change `Box<dyn LLM>` fields → `Arc<dyn LLM>` in all chain structs
4. Change `From<L> for Box<dyn LLM>` → `From<L> for Arc<dyn LLM>`
5. Replace all `llm.clone_box()` call sites with `Arc::clone(&llm)`
6. Update builder `.llm()` method signatures to accept `impl Into<Arc<dyn LLM>>`
   </migration>
