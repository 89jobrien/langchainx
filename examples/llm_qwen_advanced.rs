use futures::StreamExt;
use langchainx::{
    language_models::{llm::LLM, options::CallOptions},
    llm::{Qwen, QwenModel},
    schemas::Message,
};
use std::{env, io::Write};

#[tokio::main]
async fn main() {
    // Get API key from environment variable
    let api_key = env::var("QWEN_API_KEY").expect("QWEN_API_KEY environment variable must be set");
    // Example 1: Basic generation with options
    println!("=== Example 1: Basic Generation with Options ===");
    let qwen = Qwen::new()
        .with_api_key(api_key.clone())
        .with_model(QwenModel::QwenTurbo.to_string())
        .with_options(
            CallOptions::default()
                .with_max_tokens(500)
                .with_temperature(0.7)
                .with_top_p(0.9),
        );

    // Create a system and user message
    let messages = vec![
        Message::new_system_message("You are a helpful AI assistant who responds in Chinese."),
        Message::new_human_message(
            "What are the three most popular programming languages in 2023?",
        ),
    ];

    let response = qwen.generate(&messages).await.unwrap();
    println!("Response: {}", response.generation);
    println!("Tokens used: {:?}", response.tokens);
    println!("\n");

    // Example 2: Streaming response
    println!("=== Example 2: Streaming Response ===");

    let streaming_qwen = Qwen::new()
        .with_api_key(api_key.clone())
        .with_model(QwenModel::QwenPlus.to_string())
        .with_options(CallOptions::default().with_max_tokens(100));

    let stream_messages = vec![Message::new_human_message(
        "Write a short poem about artificial intelligence.",
    )];

    println!("Streaming response:");
    let mut stream = streaming_qwen.stream(&stream_messages).await.unwrap();
    while let Some(result) = stream.next().await {
        match result {
            Ok(data) => {
                print!("{}", data.content);
                let _ = std::io::stdout().flush();
            }
            Err(e) => eprintln!("Stream error: {:?}", e),
        }
    }
    println!();
}
