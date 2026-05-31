# langchainx-macros: Composable DSL Macros

## Vision

Reduce langchainx wiring from imperative builder chains to declarative
one-liners. The macro crate already ships `tool!`; this doc covers
`llm!`, `chain!`, and `prompt!`.

## Current State (verbose)

```rust
let llm = OpenAI::default().with_model("gpt-4o-mini");
let prompt = HumanMessagePromptTemplate::new(
    template_fstring!("Capital of {country}?", "country"),
);
let chain = LLMChainBuilder::new()
    .prompt(prompt)
    .llm(llm)
    .build()
    .unwrap();
let answer = chain.invoke(prompt_args! { "country" => "France" }).await?;
```

## Target State (with macros)

```rust
let llm = llm!(OpenAI, model = "gpt-4o-mini");
let prompt = prompt!("Capital of {country}?", "country");
let chain = chain!(prompt, llm);
let answer = chain.invoke(prompt_args! { "country" => "France" }).await?;
```

Or fully inline:

```rust
let chain = chain!(
    prompt!("Capital of {country}?", "country"),
    llm!(OpenAI, model = "gpt-4o-mini"),
);
```

---

## Macro Specifications

### `llm!` -- LLM backend constructor

**Syntax:**

```rust
llm!(Backend)                              // Backend::default()
llm!(Backend, model = "name")             // .with_model("name")
llm!(Backend, model = "name", key = val)  // chained .with_key(val)
```

**Supported backends:** `OpenAI`, `Claude`, `Deepseek`, `Ollama`

**Expansion:**

```rust
llm!(Claude, model = "claude-sonnet-4-20250514", max_tokens = 2048)
// expands to:
Claude::default().with_model("claude-sonnet-4-20250514").with_max_tokens(2048)
```

**Design notes:**

- Each `key = val` pair maps to `.with_key(val)` -- the macro doesn't
  need to know the backend's API, it just chains builder methods.
- The zero-arg form `llm!(OpenAI)` expands to `OpenAI::default()`.
- Type inference handles the rest; no backend-specific branches needed.

### `prompt!` -- Template + message wrapper

**Syntax:**

```rust
prompt!("Tell me about {topic}", "topic")
prompt!("Hello {name}, welcome to {place}!", "name", "place")
```

**Expansion:**

```rust
prompt!("Tell me about {topic}", "topic")
// expands to:
HumanMessagePromptTemplate::new(template_fstring!("Tell me about {topic}", "topic"))
```

**Design notes:**

- Wraps the existing `template_fstring!` + `HumanMessagePromptTemplate`
  combo that appears in nearly every example.
- The `template_fstring!` macro already exists in langchainx-prompt.
- For system messages or multi-message prompts, users fall back to
  `message_formatter!` -- this macro covers the 80% case.

### `chain!` -- Wire prompt + LLM into a chain

**Syntax:**

```rust
chain!(prompt, llm)                        // basic LLMChain
chain!(prompt, llm, output_key = "result") // custom output key
```

**Expansion:**

```rust
chain!(prompt, llm)
// expands to:
LLMChainBuilder::new()
    .prompt(prompt)
    .llm(llm)
    .build()
    .expect("chain! build failed -- prompt and llm are required")
```

**Design notes:**

- Panics on build failure (like `vec![]` panics on alloc failure).
  The builder only fails if prompt or llm are missing, which can't
  happen when both are provided as macro args.
- Optional `output_key` maps to `.output_key("result")`.
- Does NOT cover ConversationalChain, SequentialChain, or
  StuffDocuments -- those have enough unique config that a macro
  would just be a worse builder. This targets the common case.

### `tool!` -- Already implemented

**Syntax:**

```rust
tool!(Name, "description", |input| body)
tool!(Name, "description", parameters = json!({...}), |input| body)
```

Generates a unit struct + full `Tool` trait impl.

---

## Composability

The macros are designed to nest:

```rust
let chain = chain!(
    prompt!("Summarize: {text}", "text"),
    llm!(Claude, model = "claude-sonnet-4-20250514"),
);

let result = chain.invoke(prompt_args! { "text" => doc }).await?;
```

Agent construction stays explicit (too many knobs for a macro):

```rust
let agent = OpenAiToolAgentBuilder::new()
    .tools(&[tool1, tool2])
    .llm(llm!(OpenAI, model = "gpt-4o"))
    .build()?;
```

---

## Implementation Plan

### Phase 1: `llm!` and `prompt!`

- Add to `crates/langchainx-macros/src/lib.rs`
- Re-export from facade crate
- Both are pure `macro_rules!` -- no proc-macro needed
- `llm!` needs no new deps (just generates builder calls)
- `prompt!` re-exports `template_fstring!` and
  `HumanMessagePromptTemplate` as hidden deps

### Phase 2: `chain!`

- Depends on `LLMChainBuilder` from langchainx-chain
- Re-export as hidden dep in langchainx-macros
- Add optional `output_key` variant

### Phase 3: Examples migration

- Update examples/ to use the new macros
- Keep old builder syntax in docs as "advanced" usage
- Dual-show both styles in README

---

## Non-Goals

- Agent construction macro (too many variants, memory, tools, etc.)
- Loader macros (each loader has unique I/O setup)
- VectorStore macros (database config is inherently verbose)
- Replacing existing macros (`prompt_args!`, `template_fstring!`,
  `message_formatter!`) -- the new macros compose with them.

## Open Questions

1. Should `chain!` return `Result` instead of panicking? Leaning no --
   the inputs guarantee success, and `chain!(...)?` is noisier than
   the builder it replaces.
2. Should `llm!` accept env-var resolution for API keys?
   e.g. `llm!(Claude, api_key = env!("ANTHROPIC_API_KEY"))` --
   this already works since `env!()` is a standard macro that
   evaluates at compile time.
