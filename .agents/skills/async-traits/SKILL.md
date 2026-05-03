---
name: async-traits
description: JOB-252 — async_trait overhead, native async fn in traits (Rust 1.75+), and when each applies.
---

<oneliner>
async_trait boxes every future (heap alloc per call). Native async fn in traits is stable
since Rust 1.75 for static dispatch. dyn Trait still needs boxing — use async_trait only
at dyn call sites.
</oneliner>

<problem>
## Current Problem (JOB-252)

Every trait in this codebase uses `#[async_trait]`:

```rust
#[async_trait]
pub trait LLM: Sync + Send {
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError>;
    // desugars to:
    // fn generate(...) -> Pin<Box<dyn Future<Output=Result<...>> + Send + 'async_trait>>
}
```

This means **every** `generate()` call heap-allocates a `Box<dyn Future>`. With 124 uses
across 60 files, this is pervasive.
</problem>

<native-async>
## Native async fn in Traits (Rust 1.75+)

For static dispatch (generics), native async fn works without any macro:

```rust
// No #[async_trait] needed — works for static dispatch
pub trait LLM: Sync + Send {
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError>;
    async fn stream(&self, messages: &[Message])
        -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError>;
}

// Generic function — zero-cost, no boxing
async fn run_chain<L: LLM>(llm: &L, messages: &[Message]) -> Result<String, LLMError> {
    Ok(llm.generate(messages).await?.generation)
}
```

</native-async>

<dyn-limitation>
## dyn Trait Limitation

`async fn` in `dyn Trait` still requires boxing as of Rust 1.85. At `dyn` call sites, use
a local adapter:

```rust
use async_trait::async_trait;

// Thin wrapper that adds boxing only at the dyn boundary
#[async_trait]
trait DynLLM: Send + Sync {
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError>;
}

// Blanket impl bridges native LLM → DynLLM
impl<L: LLM + Send + Sync> DynLLM for L {
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError> {
        LLM::generate(self, messages).await
    }
}
```

Or keep `Arc<dyn LLM>` with `#[async_trait]` on just the trait definition — boxing happens
once at the trait boundary, not in every impl.
</dyn-limitation>

<current-guidance>
## Current Guidance (until JOB-252 is resolved)

- DO add `#[async_trait]` to new trait impls — consistent with existing code
- DO NOT add new `async_trait` uses to non-trait code (closures, free functions)
- When implementing JOB-252, start with `LLM` trait (highest call frequency), measure, then
  proceed to `Tool`, `Chain`, `Embedder`
  </current-guidance>
