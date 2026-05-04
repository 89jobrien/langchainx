/// Tier 2 — local LLM e2e tests (Ollama).
///
/// Tests skip automatically when Ollama is not running or the model is not
/// pulled. Run `ollama pull qwen2.5:0.5b` to enable them.
///
/// Assertions verify **doneness** (chain completed, non-empty output, no panic)
/// not the specific content of the model's response.
mod common;

use langchainx::{
    chain::{Chain, LLMChainBuilder, conversational::builder::ConversationalChainBuilder},
    embedding::OllamaEmbedder,
    fmt_template,
    llm::ollama::client::Ollama,
    memory::SimpleMemory,
    message_formatter,
    prompt::HumanMessagePromptTemplate,
    prompt_args, template_fstring,
};

use common::ollama_available;

const MODEL: &str = "qwen2.5:0.5b";
const EMBED_MODEL: &str = "nomic-embed-text-v2-moe";

// ---------------------------------------------------------------------------
// LLMChain — real generation completes without error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ollama_llm_chain_doneness() {
    if !ollama_available(MODEL).await {
        eprintln!("SKIP: ollama model {MODEL} not available");
        return;
    }

    let llm = Ollama::default().with_model(MODEL);

    let prompt = message_formatter![fmt_template!(HumanMessagePromptTemplate::new(
        template_fstring!("{input}", "input")
    ))];

    let chain = LLMChainBuilder::new()
        .llm(llm)
        .prompt(prompt)
        .build()
        .unwrap();

    let result = chain
        .invoke(prompt_args! { "input" => "Reply with a single word: hello" })
        .await
        .unwrap();

    assert!(!result.is_empty(), "model returned no output");
}

// ---------------------------------------------------------------------------
// LLMChain stream — chunks arrive, stream completes without error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ollama_llm_chain_stream_doneness() {
    if !ollama_available(MODEL).await {
        eprintln!("SKIP: ollama model {MODEL} not available");
        return;
    }

    use futures::StreamExt;

    let llm = Ollama::default().with_model(MODEL);

    let prompt = message_formatter![fmt_template!(HumanMessagePromptTemplate::new(
        template_fstring!("{input}", "input")
    ))];

    let chain = LLMChainBuilder::new()
        .llm(llm)
        .prompt(prompt)
        .build()
        .unwrap();

    let mut stream = chain
        .stream(prompt_args! { "input" => "Say one word" })
        .await
        .unwrap();

    let mut chunks = 0usize;
    let mut content = String::new();
    while let Some(chunk) = stream.next().await {
        let data = chunk.unwrap();
        content.push_str(&data.content);
        chunks += 1;
    }

    assert!(chunks > 0, "stream produced no chunks");
    assert!(!content.is_empty(), "stream produced no content");
}

// ---------------------------------------------------------------------------
// ConversationalChain — multi-turn session completes all turns
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ollama_conversational_chain_multi_turn() {
    if !ollama_available(MODEL).await {
        eprintln!("SKIP: ollama model {MODEL} not available");
        return;
    }

    let llm = Ollama::default().with_model(MODEL);
    let memory = SimpleMemory::new();

    let chain = ConversationalChainBuilder::new()
        .llm(llm)
        .memory(memory.into())
        .build()
        .unwrap();

    let r1 = chain
        .invoke(prompt_args! { "input" => "My name is TestUser." })
        .await
        .unwrap();

    let r2 = chain
        .invoke(prompt_args! { "input" => "What did I just tell you?" })
        .await
        .unwrap();

    // Verify doneness: both turns returned non-empty output
    assert!(!r1.is_empty(), "turn 1 returned no output");
    assert!(!r2.is_empty(), "turn 2 returned no output");
}

// ---------------------------------------------------------------------------
// OllamaEmbedder — single query returns a non-empty vector
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ollama_embed_query_doneness() {
    if !ollama_available(EMBED_MODEL).await {
        eprintln!("SKIP: ollama model {EMBED_MODEL} not available");
        return;
    }

    use langchainx::embedding::embedder_trait::Embedder;

    let embedder = OllamaEmbedder::default().with_model(EMBED_MODEL);
    let vector = embedder
        .embed_query("Why is the sky blue?")
        .await
        .expect("embed_query failed");

    assert!(!vector.is_empty(), "embed_query returned empty vector");
}

// ---------------------------------------------------------------------------
// OllamaEmbedder — batch embed returns one vector per document
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ollama_embed_documents_doneness() {
    if !ollama_available(EMBED_MODEL).await {
        eprintln!("SKIP: ollama model {EMBED_MODEL} not available");
        return;
    }

    use langchainx::embedding::embedder_trait::Embedder;

    let embedder = OllamaEmbedder::default().with_model(EMBED_MODEL);
    let docs = vec![
        "rust is a systems programming language".to_string(),
        "embeddings map text into vector space".to_string(),
        "ollama runs models locally".to_string(),
    ];

    let vectors = embedder
        .embed_documents(&docs)
        .await
        .expect("embed_documents failed");

    assert_eq!(
        vectors.len(),
        docs.len(),
        "wrong number of embeddings returned"
    );
    for (i, v) in vectors.iter().enumerate() {
        assert!(!v.is_empty(), "embedding {i} is empty");
        assert_eq!(
            v.len(),
            vectors[0].len(),
            "embedding {i} has inconsistent dimensions"
        );
    }
}
