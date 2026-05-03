---
name: memory-construction
description: JOB-256 — blanket From impl, removing redundant Into impls, RwLock vs Mutex for memory.
---

<oneliner>
All three memory types have identical Into<Arc<Mutex<dyn BaseMemory>>> boilerplate.
A blanket impl will replace them. Use .into() now — the call site will not change.
</oneliner>

<current-boilerplate>
## Current Boilerplate (JOB-256)

This 10-line block is copy-pasted across SimpleMemory, WindowBufferMemory, DummyMemory:

```rust
impl Into<Arc<Mutex<dyn BaseMemory>>> for SimpleMemory {
    fn into(self) -> Arc<Mutex<dyn BaseMemory>> {
        Arc::new(Mutex::new(self))
    }
}
// repeated for DummyMemory, WindowBufferMemory
```

Also: `Into<Arc<dyn BaseMemory>>` (non-mutex) is implemented on all three but never used
by any chain or executor — it is dead API.
</current-boilerplate>

<future-blanket>
## Future Blanket Impl (JOB-256 target)

```rust
// Will replace the 6 manual impls (3 types × 2 variants)
impl<T: BaseMemory + Send + Sync + 'static> From<T> for Arc<Mutex<dyn BaseMemory>> {
    fn from(mem: T) -> Self {
        Arc::new(Mutex::new(mem))
    }
}
```

Call sites using `.into()` will continue to work unchanged.
</future-blanket>

<adding-new-memory>
## Adding a New Memory Type

Until JOB-256 is merged, implement the conversions manually:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use langchainx::schemas::memory::BaseMemory;

pub struct SlidingWindowMemory { /* ... */ }

impl BaseMemory for SlidingWindowMemory { /* ... */ }

// Required until blanket impl lands
impl Into<Arc<Mutex<dyn BaseMemory>>> for SlidingWindowMemory {
    fn into(self) -> Arc<Mutex<dyn BaseMemory>> {
        Arc::new(Mutex::new(self))
    }
}

// DO NOT add Into<Arc<dyn BaseMemory>> — it is dead API
```

</adding-new-memory>

<rwlock-note>
## Mutex vs RwLock

All memory uses `tokio::sync::Mutex` today. Memory is read far more often than written
(read on every plan() call, write only on Finish). `RwLock` would allow concurrent readers.

JOB-256 also evaluates switching to `RwLock`. Do not change this unilaterally — it is
part of the JOB-256 scope.
</rwlock-note>
