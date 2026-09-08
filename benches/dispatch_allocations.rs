//! Deterministic allocation regression for native and dynamic LLM dispatch.
//!
//! Run with `cargo bench --bench dispatch_allocations`.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use futures::Stream;
use langchainx::{
    language_models::{GenerateResult, LLMError, llm::LLM},
    schemas::{Message, StreamData},
};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Delegating the allocation unchanged preserves `System`'s contract.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: `pointer` and `layout` came from the delegated `System` allocator.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Delegating the allocation unchanged preserves `System`'s contract.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: The arguments satisfy the delegated `System` allocator contract.
        let pointer = unsafe { System.realloc(pointer, layout, new_size) };
        if !pointer.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        pointer
    }
}

struct ReadyLlm;

impl LLM for ReadyLlm {
    async fn generate(&self, _messages: &[Message]) -> Result<GenerateResult, LLMError> {
        Ok(GenerateResult::default())
    }

    async fn stream(
        &self,
        _messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
        Ok(Box::pin(futures::stream::empty()))
    }
}

fn measured_allocations(operation: impl FnOnce()) -> usize {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    operation();
    ALLOCATIONS.load(Ordering::SeqCst)
}

fn main() {
    const ITERATIONS: usize = 10_000;

    let concrete = ReadyLlm;
    let dynamic: Arc<dyn langchainx::language_models::llm::DynLLM> = Arc::new(ReadyLlm);

    let static_allocations = measured_allocations(|| {
        for _ in 0..ITERATIONS {
            let _future = black_box(LLM::generate(&concrete, &[]));
        }
    });
    let dynamic_allocations = measured_allocations(|| {
        for _ in 0..ITERATIONS {
            let _future = black_box(dynamic.dyn_generate(&[]));
        }
    });

    assert_eq!(
        static_allocations, 0,
        "static dispatch must not box futures"
    );
    assert_eq!(
        dynamic_allocations, ITERATIONS,
        "dynamic dispatch must allocate exactly one future box per call"
    );

    println!("static native dispatch: {static_allocations} allocations/{ITERATIONS} calls");
    println!("boxed dynamic dispatch: {dynamic_allocations} allocations/{ITERATIONS} calls");
}
