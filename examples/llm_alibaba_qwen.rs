use langchainx::language_models::llm::LLM;
use langchainx::llm::Qwen;
use langchainx::schemas::Message;

#[tokio::main]
async fn main() {
    // Initialize the Qwen client
    // Requires QWEN_API_KEY environment variable to be set
    let api_key =
        std::env::var("QWEN_API_KEY").expect("QWEN_API_KEY environment variable must be set");
    let qwen = Qwen::new()
        .with_api_key(api_key)
        .with_model("qwen-turbo"); // Can use enum: QwenModel::QwenTurbo.to_string()

    // Generate a response
    let response = qwen
        .generate(&[Message::new_human_message("Introduce the Great Wall")])
        .await
        .unwrap();

    println!("Response: {}", response.generation);
}
