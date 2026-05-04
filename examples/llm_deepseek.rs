use langchainx::language_models::llm::LLM;
use langchainx::llm::Deepseek;
use langchainx::schemas::Message;

#[tokio::main]
async fn main() {
    // Initialize the Deepseek client
    // Requires DEEPSEEK_API_KEY environment variable to be set
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .expect("DEEPSEEK_API_KEY environment variable must be set");
    let deepseek = Deepseek::new()
        .with_api_key(api_key)
        .with_model("deepseek-chat"); // Can use enum: DeepseekModel::DeepseekChat.to_string()

    // Generate a response
    let response = deepseek
        .generate(&[Message::new_human_message("Introduce the Great Wall")])
        .await
        .unwrap();

    println!("Response: {}", response.generation);
}
