---
name: streaming
description: JOB-253 — correct streaming pattern via stream() return value; removing streaming_func side-channel.
---

<oneliner>
Use llm.stream() or chain.stream() — they return Pin<Box<dyn Stream>>. Never use
streaming_func in CallOptions; that is a deprecated side-channel that will be removed (JOB-253).
</oneliner>

<correct-pattern>
## Correct Streaming Pattern

```rust
use futures::StreamExt;
use langchain_rust::{
    chain::{Chain, LLMChainBuilder},
    prompt_args,
};

let chain = LLMChainBuilder::new().llm(llm).prompt(prompt).build()?;

let mut stream = chain.stream(prompt_args! { "input" => "Write a poem" }).await?;

while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(data) => print!("{}", data.content),
        Err(e) => eprintln!("error: {e}"),
    }
}
println!(); // final newline
```

</correct-pattern>

<llm-stream>
## LLM-Level Streaming

```rust
use langchain_rust::language_models::llm::LLM;
use langchain_rust::schemas::Message;

let mut stream = llm.stream(&[Message::new_human_message("Hello")]).await?;
while let Some(chunk) = stream.next().await {
    let data = chunk?;
    if !data.content.is_empty() {
        print!("{}", data.content);
    }
}
```

</llm-stream>

<anti-pattern>
## Anti-Pattern: streaming_func (JOB-253 — DO NOT USE)

```rust
// WRONG — streaming_func is a side-channel callback in CallOptions
// It will be removed in JOB-253. Do not use in new code.
let options = CallOptions::new()
    .with_streaming_func(|chunk| async move {
        print!("{chunk}");
        Ok(())
    });
let llm = OpenAI::default().with_options(options);
let result = llm.generate(&messages).await?; // side-effects in callback, result separate
```

Problems with `streaming_func`:

- Runtime state inside a config struct
- `Arc<Mutex<FnMut>>` contamination
- Result (full string) and display (callback) are completely separate
- Cannot be tested without an async closure mock
  </anti-pattern>

<stream-data>
## StreamData Fields

```rust
pub struct StreamData {
    pub value: Value,      // raw JSON from the provider
    pub tokens: Option<TokenUsage>,
    pub content: String,   // the text chunk — this is what you usually want
}
```

</stream-data>
