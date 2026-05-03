---
name: memory
description: BaseMemory trait, memory types (SimpleMemory, WindowBufferMemory, DummyMemory), and Arc<Mutex> construction patterns.
---

<oneliner>
Memory is Arc<Mutex<dyn BaseMemory>>. Use SimpleMemory for basic history, WindowBufferMemory
for a sliding window. Pass .into() to convert — all types implement the conversion.
</oneliner>

<base-memory-trait>
## BaseMemory Trait

```rust
pub trait BaseMemory: Send + Sync {
    fn messages(&self) -> Vec<Message>;
    fn add_message(&mut self, message: Message);
    fn add_user_message(&mut self, message: impl Into<String>);  // convenience
    fn add_ai_message(&mut self, message: impl Into<String>);    // convenience
    fn clear(&mut self);
}
```

</base-memory-trait>

<memory-types>
## Memory Types

```rust
use langchainx::memory::{SimpleMemory, WindowBufferMemory, DummyMemory};

// Unbounded history
let mem = SimpleMemory::new();

// Sliding window — keeps last N exchanges
let mem = WindowBufferMemory::new(5);

// No-op — for chains that require memory param but don't need it
let mem = DummyMemory::new();
```

</memory-types>

<construction>
## Construction: Into<Arc<Mutex<dyn BaseMemory>>>

All memory types implement `Into<Arc<Mutex<dyn BaseMemory>>>`. Use `.into()`:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use langchainx::{
    memory::SimpleMemory,
    schemas::memory::BaseMemory,
};

// Preferred — .into() calls Arc::new(Mutex::new(self))
let mem: Arc<Mutex<dyn BaseMemory>> = SimpleMemory::new().into();

// Equivalent explicit form
let mem = Arc::new(Mutex::new(SimpleMemory::new())) as Arc<Mutex<dyn BaseMemory>>;
```

Pass to chain/agent builders:

```rust
ConversationalChainBuilder::new()
    .memory(SimpleMemory::new().into())
    ...
```

</construction>

<reading-memory>
## Reading from Memory

```rust
let mem: Arc<Mutex<dyn BaseMemory>> = SimpleMemory::new().into();

// Lock to read
let guard = mem.lock().await;
let messages = guard.messages();
drop(guard);  // release lock before any await

// Lock to write
let mut guard = mem.lock().await;
guard.add_user_message("Hello");
guard.add_ai_message("Hi there!");
```

Never hold the lock across an `.await` — use a separate scope or drop before await points.
</reading-memory>
