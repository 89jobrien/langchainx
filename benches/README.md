# Dispatch Allocation Regression

Run the deterministic allocation measurement with:

```bash
cargo bench --bench dispatch_allocations
```

The benchmark constructs 10,000 `LLM::generate` futures through each dispatch path. Static
native dispatch must allocate zero times; the object-safe `DynLLM` boundary must allocate one
boxed future per call. It checks allocation counts rather than wall-clock timing.
