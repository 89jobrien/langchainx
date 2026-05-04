/// ci_fix_agent — compile-only example showing how to wire coding_tools() into an AgentExecutor.
///
/// This demonstrates the tool suite setup. No live LLM call is made.
///
/// Run:
///   cargo run --example ci_fix_agent --all-features
use langchainx_tools::code::coding_tools;

fn main() {
    let tools = coding_tools(".");
    println!("Coding tools available ({} total):", tools.len());
    for tool in &tools {
        println!("  - {} : {}", tool.name(), tool.description());
    }
}
