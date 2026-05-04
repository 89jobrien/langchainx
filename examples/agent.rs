use langchainx_agent::{AgentExecutor, ConversationalAgentBuilder};
use langchainx_chain::{Chain, options::ChainCallOptions};
use langchainx_llm::OpenAI;
use langchainx_memory::SimpleMemory;
use langchainx_prompt::prompt_args;
use langchainx_tools::CommandExecutor;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let llm = OpenAI::default().with_model("gpt-4o");
    let memory = SimpleMemory::new();
    let command_executor = CommandExecutor::default();
    let agent = ConversationalAgentBuilder::new()
        .tools(&[Arc::new(command_executor)])
        .options(ChainCallOptions::new().with_max_tokens(1000))
        .build(llm)
        .unwrap();

    let executor = AgentExecutor::from_agent(agent).with_memory(memory.into());

    let input_variables = prompt_args! {
        "input" => "What is the name of the current dir",
    };

    match executor.invoke(input_variables).await {
        Ok(result) => {
            println!("Result: {:?}", result);
        }
        Err(e) => panic!("Error invoking LLMChain: {:?}", e),
    }
}
