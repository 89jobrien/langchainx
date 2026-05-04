---
name: arc-dyn-llm
description: Arc<dyn LLM> ownership model, IntoArcLLM conversion trait, and builder patterns.
---

<oneliner>
LLMClone is gone. Use Arc<dyn LLM> for shared ownership — Arc is Clone by definition.
IntoArcLLM accepts both concrete types and Arc<dyn LLM> in builder .llm() methods.
</oneliner>

<current-state>
## Current State

`LLMClone` and `clone_box()` have been removed. The `IntoArcLLM` conversion trait is in
`src/language_models/llm.rs`:

```rust
pub trait IntoArcLLM {
    fn into_arc_llm(self) -> Arc<dyn LLM>;
}

impl<L: LLM + 'static> IntoArcLLM for L {
    fn into_arc_llm(self) -> Arc<dyn LLM> { Arc::new(self) }
}

impl IntoArcLLM for Arc<dyn LLM> {
    fn into_arc_llm(self) -> Arc<dyn LLM> { self }
}
```

Builder `.llm()` methods accept `impl IntoArcLLM` — pass either a concrete LLM or an
`Arc<dyn LLM>` directly.
</current-state>

<preferred-pattern>
## Preferred Pattern: Arc<dyn LLM>

```rust
use std::sync::Arc;
use langchainx::language_models::llm::LLM;

// Cheap clone — just an atomic ref count increment
let llm: Arc<dyn LLM> = Arc::new(Claude::new());

// Both chains share the same LLM instance
let stuff_chain = StuffDocumentBuilder::new().llm(Arc::clone(&llm)).build()?;
let condense_chain = ConversationalRetrieverChainBuilder::new().llm(Arc::clone(&llm));
```

</preferred-pattern>

<concrete-llm>
## Passing a Concrete LLM to a Builder

Builders accept concrete types directly via `IntoArcLLM`:

```rust
// Concrete type — builder wraps it in Arc internally
let chain = LLMChainBuilder::new()
    .llm(OpenAI::default())
    .prompt(prompt)
    .build()?;

// Arc<dyn LLM> — also accepted
let llm: Arc<dyn LLM> = Arc::new(OpenAI::default());
let chain = LLMChainBuilder::new()
    .llm(llm)
    .prompt(prompt)
    .build()?;
```

</concrete-llm>
