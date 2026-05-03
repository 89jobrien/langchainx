---
name: chains
description: All Chain types — LLMChain, ConversationalChain, SequentialChain, StuffDocuments, ConversationalRetrievalQA, SqlDatabaseChain.
---

<oneliner>
Every chain type has a dedicated Builder. Use invoke() for a string result, call() for token
usage, stream() for streaming. ConversationalChain needs memory; retrieval chains need a Retriever.
</oneliner>

<chain-types>
## Chain Types

| Chain                          | Builder                               | Requires                                        |
| ------------------------------ | ------------------------------------- | ----------------------------------------------- |
| `LLMChain`                     | `LLMChainBuilder`                     | prompt, llm                                     |
| `ConversationalChain`          | `ConversationalChainBuilder`          | llm (memory optional, defaults to SimpleMemory) |
| `SequentialChain`              | `SequentialChainBuilder`              | chains vec, input/output key mapping            |
| `StuffDocumentsChain`          | `StuffDocumentBuilder`                | llm (or a prompt)                               |
| `ConversationalRetrieverChain` | `ConversationalRetrieverChainBuilder` | llm, retriever                                  |
| `SqlDatabaseChain`             | `SqlDatabaseChainBuilder`             | llm, datasource                                 |

</chain-types>

<conversational>
## ConversationalChain

```rust
use langchainx::{
    chain::{Chain, ConversationalChainBuilder},
    llm::openai::OpenAI,
    memory::SimpleMemory,
    prompt_args,
};

let chain = ConversationalChainBuilder::new()
    .llm(OpenAI::default())
    .memory(SimpleMemory::new().into())  // Arc<Mutex<dyn BaseMemory>>
    .build()?;

// Turn 1
chain.invoke(prompt_args! { "input" => "My name is Alice" }).await?;

// Turn 2 — chain remembers history
let reply = chain.invoke(prompt_args! { "input" => "What is my name?" }).await?;
```

The default input key is `"input"`. History is stored under `"history"` in the prompt.
</conversational>

<sequential>
## SequentialChain

```rust
use langchainx::chain::{SequentialChainBuilder, Chain};

let chain = SequentialChainBuilder::new()
    .add_chain(chain1)   // Box<dyn Chain>
    .add_chain(chain2)
    .build()?;

// Output of chain1 is automatically piped as input to chain2
// using the output_key of chain1 as an input key for chain2
```

</sequential>

<stuff-documents>
## StuffDocumentsChain

Stuffs retrieved documents into the prompt context.

```rust
use langchainx::chain::StuffDocumentBuilder;

let chain = StuffDocumentBuilder::new()
    .llm(OpenAI::default())
    .build()?;
```

Used internally by `ConversationalRetrieverChain` and `question_answering::load_qa_with_sources_chain`.
</stuff-documents>

<retrieval-qa>
## ConversationalRetrievalQA

```rust
use langchainx::{
    chain::{Chain, ConversationalRetrieverChainBuilder},
    llm::openai::OpenAI,
    memory::SimpleMemory,
    vectorstore::Retriever,
    prompt_args,
};

let retriever = Retriever::new(vector_store, 5);

let chain = ConversationalRetrieverChainBuilder::new()
    .llm(OpenAI::default())
    .retriever(retriever)
    .memory(SimpleMemory::new().into())
    .build()?;

let answer = chain.invoke(prompt_args! {
    "input" => "What does the document say about X?"
}).await?;
```

</retrieval-qa>

<streaming-chain>
## Streaming

```rust
use futures::StreamExt;

let mut stream = chain.stream(prompt_args! { "input" => "Tell me a story" }).await?;
while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(data) => print!("{}", data.content),
        Err(e) => eprintln!("stream error: {e}"),
    }
}
```

Note: chains with memory do NOT automatically save memory when using `stream()` — only
`call()` and `invoke()` save to memory.
</streaming-chain>
