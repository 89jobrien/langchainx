---
name: fundamentals
description: Core langchain-rust patterns — LLMChain, builder pattern, Chain trait, prompt macros, and basic invocation.
---

<oneliner>
Build chains with LLMChainBuilder, use prompt_args! for input, call chain.invoke() or
chain.call() for output. The builder pattern is universal across all chain types.
</oneliner>

<builder-pattern>
## Builder Pattern

Every chain type uses a `*Builder` struct with method chaining and a `.build()` that returns
`Result<Chain, ChainError>`. Required fields cause a `ChainError::MissingObject` if absent.

```rust
let chain = LLMChainBuilder::new()
    .prompt(formatter)          // required
    .llm(llm)                   // required
    .options(ChainCallOptions::default())  // optional
    .output_key("answer")       // optional, default "output"
    .build()?;
```

</builder-pattern>

<prompt-macros>
## Prompt Macros

```rust
use langchain_rust::{
    message_formatter, fmt_message, fmt_template,
    prompt::{HumanMessagePromptTemplate, MessageOrTemplate},
    prompt_args, template_fstring,
    schemas::Message,
};

// Build a multi-message prompt template
let prompt = message_formatter![
    fmt_message!(Message::new_system_message("You are a helpful assistant.")),
    fmt_template!(HumanMessagePromptTemplate::new(
        template_fstring!("{input}", "input")
    )),
];

// Build input variables
let input = prompt_args! {
    "input" => "What is Rust?",
};
```

</prompt-macros>

<basic-llmchain>
## LLMChain — Basic Usage

```rust
use langchain_rust::{
    chain::{Chain, LLMChainBuilder},
    llm::openai::OpenAI,
    message_formatter, fmt_message, fmt_template,
    prompt::{HumanMessagePromptTemplate, MessageOrTemplate},
    prompt_args, template_fstring,
    schemas::Message,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm = OpenAI::default();

    let prompt = message_formatter![
        fmt_message!(Message::new_system_message("You are helpful.")),
        fmt_template!(HumanMessagePromptTemplate::new(
            template_fstring!("{input}", "input")
        )),
    ];

    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(llm)
        .build()?;

    // invoke: returns String
    let output = chain.invoke(prompt_args! { "input" => "Hello" }).await?;
    println!("{output}");

    // call: returns GenerateResult (includes token usage)
    let result = chain.call(prompt_args! { "input" => "Hello" }).await?;
    println!("{}", result.generation);
    println!("{:?}", result.tokens);

    Ok(())
}
```

</basic-llmchain>

<chain-trait>
## Chain Trait

```rust
#[async_trait]
pub trait Chain: Sync + Send {
    async fn call(&self, input_variables: PromptArgs) -> Result<GenerateResult, ChainError>;
    async fn invoke(&self, input_variables: PromptArgs) -> Result<String, ChainError>;
    async fn execute(&self, input_variables: PromptArgs) -> Result<HashMap<String, Value>, ChainError>;
    async fn stream(&self, input_variables: PromptArgs)
        -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, ChainError>> + Send>>, ChainError>;
    fn get_input_keys(&self) -> Vec<String>;
    fn get_output_keys(&self) -> Vec<String>;
}
```

- `invoke` — returns just the generated string (most common)
- `call` — returns `GenerateResult` with token usage
- `execute` — returns `HashMap<String, Value>` keyed by output key
- `stream` — returns a `Stream` of `StreamData` chunks
  </chain-trait>

<fix-missing-prompt>
## Common Mistake: Missing prompt variables

`prompt_args!` keys must exactly match the variable names in `template_fstring!`.

```rust
// WRONG: template uses "input" but args use "query"
let prompt = message_formatter![fmt_template!(
    HumanMessagePromptTemplate::new(template_fstring!("{input}", "input"))
)];
let args = prompt_args! { "query" => "Hello" }; // key mismatch — runtime error

// CORRECT: keys match
let args = prompt_args! { "input" => "Hello" };
```

</fix-missing-prompt>
