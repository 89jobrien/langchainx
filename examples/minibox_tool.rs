/// Demonstrates wiring `MiniboxTool` into an `AgentExecutor` via
/// `OpenAiToolAgentBuilder`. The agent is given a natural-language prompt
/// that exercises the ps, run, and stop actions in a single tool-use loop.
///
/// Prerequisites:
///   - `OPENAI_API_KEY` environment variable set
///   - `mbx` CLI on PATH (or adjust the builder path)
///
/// Run:
///   cargo run --example minibox_tool --all-features
use std::sync::Arc;

use langchainx::agent::{AgentExecutor, OpenAiToolAgentBuilder};
use langchainx::chain::{options::ChainCallOptions, Chain};
use langchainx::llm::OpenAI;
use langchainx::memory::SimpleMemory;
use langchainx::prompt::prompt_args;
use langchainx::tools::MiniboxTool;

#[tokio::main]
async fn main() {
    // Build MiniboxTool with defaults (uses `mbx` on PATH).
    // For a custom binary location:
    //   MiniboxTool::builder().mbx_path("/usr/local/bin/mbx").build()
    let minibox = MiniboxTool::new();

    let llm = OpenAI::default();
    let memory = SimpleMemory::new();

    let agent = OpenAiToolAgentBuilder::new()
        .tools(&[Arc::new(minibox)])
        .options(ChainCallOptions::new().with_max_tokens(1000))
        .build(llm)
        .expect("failed to build agent");

    let executor = AgentExecutor::from_agent(agent).with_memory(memory.into());

    // This prompt exercises three MiniboxTool actions in sequence:
    //   1. ps   — list running containers
    //   2. run  — start an alpine container
    //   3. stop — stop the container that was just started
    let input_variables = prompt_args! {
        "input" => "First list all running minibox containers. \
                    Then run a new alpine:latest container named 'demo'. \
                    Finally, stop the 'demo' container.",
    };

    match executor.invoke(input_variables).await {
        Ok(result) => {
            println!("Result: {}", result);
        }
        Err(e) => eprintln!("Error: {e:?}"),
    }
}
