---
name: rag
description: VectorStore trait, Embedder trait, Retriever, document loaders, and ConversationalRetrievalQA.
---

<oneliner>
RAG pipeline: load documents → embed → store in VectorStore → wrap in Retriever →
pass to ConversationalRetrieverChainBuilder. Each backend is behind a feature flag.
</oneliner>

<embedder-trait>
## Embedder Trait

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed_documents(&self, documents: &[String]) -> Result<Vec<Vec<f64>>, EmbedderError>;
    async fn embed_query(&self, text: &str) -> Result<Vec<f64>, EmbedderError>;
}
```

Available embedders (each behind a feature flag):

| Backend           | Struct              | Feature     |
| ----------------- | ------------------- | ----------- |
| OpenAI            | `OpenAiEmbedder`    | (default)   |
| Ollama            | `OllamaEmbedder`    | `ollama`    |
| FastEmbed (local) | `FastEmbed`         | `fastembed` |
| MistralAI         | `MistralAiEmbedder` | `mistralai` |

</embedder-trait>

<vectorstore-trait>
## VectorStore Trait

```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    type Options;
    async fn add_documents(&self, docs: &[Document], opt: &Self::Options)
        -> Result<Vec<String>, Box<dyn Error>>;
    async fn similarity_search(&self, query: &str, limit: usize, opt: &Self::Options)
        -> Result<Vec<Document>, Box<dyn Error>>;
}
```

Use the `add_documents!` and `similarity_search!` macros for ergonomic default options:

```rust
use langchainx::{add_documents, similarity_search};

add_documents!(store, &docs).await?;
let results = similarity_search!(store, "query text", 5).await?;
```

</vectorstore-trait>

<retriever>
## Retriever

Wraps a VectorStore for use in chains:

```rust
use langchainx::vectorstore::Retriever;

let retriever = Retriever::new(vector_store, 5);  // 5 = num docs to retrieve
```

</retriever>

<full-rag-pipeline>
## Full RAG Pipeline (Postgres/pgvector example)

```rust
// Cargo.toml: langchainx = { features = ["postgres"] }
use langchainx::{
    chain::{Chain, ConversationalRetrieverChainBuilder},
    embedding::openai::OpenAiEmbedder,
    llm::openai::OpenAI,
    memory::SimpleMemory,
    prompt_args,
    vectorstore::{pgvector::PgVectorBuilder, Retriever, VecStoreOptions},
};

let embedder = OpenAiEmbedder::default();

let store = PgVectorBuilder::new()
    .embedder(embedder)
    .connection_string("postgres://...")
    .build()
    .await?;

// Index documents
add_documents!(store, &documents).await?;

// Build retrieval chain
let retriever = Retriever::new(store, 5);
let chain = ConversationalRetrieverChainBuilder::new()
    .llm(OpenAI::default())
    .retriever(retriever)
    .memory(SimpleMemory::new().into())
    .build()?;

let answer = chain.invoke(prompt_args! {
    "input" => "Summarize the key points."
}).await?;
```

</full-rag-pipeline>

<document-loaders>
## Document Loaders

All implement `Loader` returning `Stream<Item = Result<Document, LoaderError>>`.

```rust
use futures::StreamExt;
use langchainx::document_loaders::{TextLoader, CsvLoader, HtmlLoader};

let mut stream = TextLoader::new("path/to/file.txt").load().await?;
while let Some(doc) = stream.next().await {
    let doc = doc?;
    println!("{}", doc.page_content);
}
```

Feature-gated loaders: `PdfLoader` (`lopdf`/`pdf-extract`), `GitCommitLoader` (`git`),
`HtmlToMarkdownLoader` (`html-to-markdown`), `SourceCodeLoader` (`tree-sitter`).
</document-loaders>
