---
name: agents
description: AgentExecutor, ChatAgent (ReAct), OpenAIToolsAgent, and the Agent trait.
---

<oneliner>
AgentExecutor runs the tool-use loop. Two agent types: OpenAIToolsAgent (function calling,
preferred) and ChatAgent (ReAct text parsing). Both implement Agent and are wrapped in
AgentExecutor::from_agent().
</oneliner>

<agent-trait>
## Agent Trait

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    async fn plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError>;

    fn get_tools(&self) -> Vec<Arc<dyn Tool>>;
}
```

`AgentEvent` is either `Action(Vec<AgentAction>)` or `Finish(AgentFinish)`. The executor
calls `plan()` in a loop until `Finish` or `max_iterations`.
</agent-trait>

<openai-tools-agent>
## OpenAIToolsAgent (recommended)

Uses OpenAI function-calling API. More reliable than ReAct text parsing.

```rust
use std::sync::Arc;
use langchainx::{
    agent::{AgentExecutor, OpenAIToolsAgentBuilder},
    chain::Chain,
    llm::openai::OpenAI,
    memory::SimpleMemory,
    prompt_args,
    tools::Tool,
};

let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(MyTool)];

let agent = OpenAIToolsAgentBuilder::new()
    .tools(&tools)
    .llm(OpenAI::default())
    .build()?;

let executor = AgentExecutor::from_agent(agent)
    .with_max_iterations(10)
    .with_memory(SimpleMemory::new().into())
    .with_break_if_error(false);

// AgentExecutor implements Chain — use the Chain API
let result = executor.invoke(prompt_args! {
    "input" => "How many words are in 'hello world'?",
}).await?;
println!("{result}");
```

</openai-tools-agent>

<chat-agent>
## ChatAgent (ReAct)

Uses text-based ReAct reasoning. Works with any LLM but is less reliable than function calling.

```rust
use langchainx::agent::{AgentExecutor, ChatAgentBuilder};

let agent = ChatAgentBuilder::new()
    .tools(&tools)
    .llm(OpenAI::default())
    .build()?;

let executor = AgentExecutor::from_agent(agent)
    .with_max_iterations(10);
```

</chat-agent>

<executor-options>
## AgentExecutor Options

```rust
AgentExecutor::from_agent(agent)
    .with_max_iterations(10)          // default: Some(10). None = unlimited (dangerous)
    .with_memory(mem.into())          // Arc<Mutex<dyn BaseMemory>>
    .with_break_if_error(true)        // return Err on tool failure instead of continuing
```

When `max_iterations` is reached, the executor returns `Ok(GenerateResult)` with
`generation = "Max iterations reached"` — not an error.
</executor-options>

<memory-in-executor>
## Memory in AgentExecutor

When memory is set, the executor:

1. Injects `chat_history` into `input_variables` before each `plan()` call
2. On `Finish`, saves user input, tool call messages, and AI response to memory

The required input key is `"input"` (hardcoded in executor).

```rust
let executor = AgentExecutor::from_agent(agent)
    .with_memory(SimpleMemory::new().into());

// First turn
executor.invoke(prompt_args! { "input" => "My name is Alice" }).await?;

// Second turn — executor injects chat_history automatically
executor.invoke(prompt_args! { "input" => "What is my name?" }).await?;
```

</memory-in-executor>
