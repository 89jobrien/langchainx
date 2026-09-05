---
status: done
---

# Plan: Automatic `validate_input` with Actionable Errors

## Goal

Make missing/misnamed `PromptArgs` keys self-diagnosing by enriching
`ChainError::MissingInputVariable` with expected-vs-provided context and
ensuring every `Chain` impl calls `validate_input()` consistently.

## Architecture

- Crates affected: `langchainx-chain`, `langchainx-agent`
- New traits/types: none
- Data flow: `Chain::call/invoke/stream` -> `validate_input()` ->
  `ChainError::MissingInputVariable { key, expected, provided }`

## Tech Stack

- Rust edition 2024, `thiserror` for error derives
- No new dependencies

## Tasks

### Task 1: Change `MissingInputVariable` to structured variant

**Crate**: `langchainx-chain`
**File(s)**: `crates/langchainx-chain/src/error.rs`
**Run**: `cargo check -p langchainx-chain 2>&1 | head -40`

1. Edit the error variant:

   ```rust
   #[error("Missing input variable `{key}`: expected {expected:?}, got {provided:?}")]
   MissingInputVariable {
       key: String,
       expected: Vec<String>,
       provided: Vec<String>,
   },
   ```

2. Verify: `cargo check -p langchainx-chain` -- expected: compile errors in
   files constructing `MissingInputVariable(String)`. This is correct; we
   fix them in subsequent tasks.

### Task 2: Update `validate_input` default impl

**Crate**: `langchainx-chain`
**File(s)**: `crates/langchainx-chain/src/chain_trait.rs`
**Run**: `cargo check -p langchainx-chain 2>&1 | head -40`

1. Replace the `validate_input` body:

   ```rust
   fn validate_input(&self, input_variables: &PromptArgs) -> Result<(), ChainError> {
       let required = self.required_keys();
       for key in &required {
           if !input_variables.contains_key(key) {
               return Err(ChainError::MissingInputVariable {
                   key: key.clone(),
                   expected: required.clone(),
                   provided: input_variables.keys().cloned().collect(),
               });
           }
       }
       Ok(())
   }
   ```

2. Verify: still compile errors from other files -- expected.

### Task 3: Fix `LLMChain` -- add validate to `stream`

**Crate**: `langchainx-chain`
**File(s)**: `crates/langchainx-chain/src/llm_chain.rs`
**Run**: `cargo nextest run -p langchainx-chain -- llm_chain`

1. In `LLMChain::stream`, add as first line:

   ```rust
   self.validate_input(&input_variables)?;
   ```

2. Update the test `missing_input_variable_returns_error` match pattern:

   ```rust
   assert!(result.is_err());
   let err = result.unwrap_err();
   match &err {
       ChainError::MissingInputVariable { key, expected, provided } => {
           assert_eq!(key, "input");
           assert!(expected.contains(&"input".to_string()));
           assert!(provided.contains(&"wrong_key".to_string()));
       }
       other => panic!("expected MissingInputVariable, got: {other:?}"),
   }
   ```

3. Add a new test verifying stream also validates:

   ```rust
   #[tokio::test]
   async fn stream_validates_input_keys() {
       let chain = make_chain(vec!["x".into()]);
       let result = chain.stream(prompt_args! { "wrong" => "val" }).await;
       assert!(matches!(
           result,
           Err(ChainError::MissingInputVariable { ref key, .. }) if key == "input"
       ));
   }
   ```

4. Verify:

   ```
   cargo nextest run -p langchainx-chain -- llm_chain  -> all green
   ```

5. Commit: `feat(chain): enrich MissingInputVariable with expected/provided keys`

### Task 4: Fix `ConversationalChain` -- use `validate_input`

**Crate**: `langchainx-chain`
**File(s)**: `crates/langchainx-chain/src/conversational/mod.rs`
**Run**: `cargo nextest run -p langchainx-chain -- conversational`

1. Add `required_keys` impl to `Chain for ConversationalChain`:

   ```rust
   fn required_keys(&self) -> Vec<String> {
       vec![self.input_key.clone()]
   }
   ```

2. In `call`, replace the manual `.get().ok_or()` on `self.input_key` with:

   ```rust
   self.validate_input(&input_variables)?;
   let input_variable = &input_variables[&self.input_key];
   ```

3. In `stream`, same replacement:

   ```rust
   self.validate_input(&input_variables)?;
   let input_variable = &input_variables[&self.input_key];
   ```

4. Update test `conversational_chain_missing_input_key_returns_error`:

   ```rust
   let result = chain.invoke(prompt_args! { "wrong_key" => "val" }).await;
   assert!(matches!(
       result,
       Err(ChainError::MissingInputVariable { ref key, .. }) if key == "input"
   ));
   ```

5. Verify:

   ```
   cargo nextest run -p langchainx-chain -- conversational  -> all green
   ```

### Task 5: Fix `StuffDocument` -- use `validate_input`

**Crate**: `langchainx-chain`
**File(s)**: `crates/langchainx-chain/src/stuff_documents/chain.rs`
**Run**: `cargo nextest run -p langchainx-chain -- stuff`

1. Add `required_keys` impl:

   ```rust
   fn required_keys(&self) -> Vec<String> {
       vec![self.input_key.clone()]
   }
   ```

2. In `call`, replace manual `.get().ok_or_else()` with:

   ```rust
   self.validate_input(&input_variables)?;
   let docs = &input_variables[&self.input_key];
   ```

3. In `stream`, same replacement.

4. Update test `missing_input_documents_key_returns_error`:

   ```rust
   assert!(
       matches!(
           result,
           Err(ChainError::MissingInputVariable { ref key, .. })
               if key == "input_documents"
       ),
       "expected MissingInputVariable error, got: {:?}",
       result
   );
   ```

5. Verify:

   ```
   cargo nextest run -p langchainx-chain -- stuff  -> all green
   ```

### Task 6: Fix `SQLDatabaseChain` -- use `validate_input`

**Crate**: `langchainx-chain`
**File(s)**: `crates/langchainx-chain/src/sql_datbase/chain.rs`
**Run**: `cargo check -p langchainx-chain`

1. Add `required_keys` impl:

   ```rust
   fn required_keys(&self) -> Vec<String> {
       vec![SQL_CHAIN_DEFAULT_INPUT_KEY_QUERY.to_string()]
   }
   ```

2. In `call_builder_chains`, replace manual `.get().ok_or_else()` with:

   ```rust
   self.validate_input(input_variables)?;
   let query = input_variables[SQL_CHAIN_DEFAULT_INPUT_KEY_QUERY].to_string();
   ```

   Note: `call_builder_chains` takes `&PromptArgs` not owned, and
   `validate_input` takes `&PromptArgs` too, so this works directly.

3. Verify: `cargo check -p langchainx-chain` -> clean (no integration
   tests to run without DB).

### Task 7: Fix `ConversationalRetrieverChain` -- use `validate_input`

**Crate**: `langchainx-chain`
**File(s)**: `crates/langchainx-chain/src/conversational_retrieval_qa/conversational_retrieval_qa.rs`
**Run**: `cargo nextest run -p langchainx-chain -- retriever`

1. Add `required_keys` impl:

   ```rust
   fn required_keys(&self) -> Vec<String> {
       vec![self.input_key.clone()]
   }
   ```

2. In `execute`, replace manual `.get().ok_or()` with:

   ```rust
   self.validate_input(&input_variables)?;
   let input_variable = &input_variables[&self.input_key];
   ```

3. In `stream`, same replacement.

4. Verify:

   ```
   cargo nextest run -p langchainx-chain -- retriever  -> all green
   ```

### Task 8: Fix `SequentialChain` -- add `required_keys`

**Crate**: `langchainx-chain`
**File(s)**: `crates/langchainx-chain/src/sequential/chain.rs`
**Run**: `cargo nextest run -p langchainx-chain -- sequential`

1. Add `required_keys` impl:

   ```rust
   fn required_keys(&self) -> Vec<String> {
       self.input_keys.iter().cloned().collect()
   }
   ```

2. In `call`, add as first line:

   ```rust
   self.validate_input(&input_variables)?;
   ```

3. In `execute`, add as first line:

   ```rust
   self.validate_input(&input_variables)?;
   ```

4. Verify:

   ```
   cargo nextest run -p langchainx-chain -- sequential  -> all green
   ```

### Task 9: Verify `AgentExecutor` compiles (no changes needed)

**Crate**: `langchainx-agent`
**File(s)**: `crates/langchainx-agent/src/executor.rs`
**Run**: `cargo nextest run -p langchainx-agent`

1. `AgentExecutor` does not construct `MissingInputVariable` directly and
   does not need `required_keys` (agent keys are dynamic). Verify it
   compiles and tests pass with no changes.

2. Verify:

   ```
   cargo nextest run -p langchainx-agent  -> all green
   ```

### Task 10: Full validation and commit

**Run**: `cargo test --all-features && cargo clippy --all-features -- -D warnings`

1. Run full test suite:

   ```
   cargo test --all-features
   ```

2. Run clippy:

   ```
   cargo clippy --all-features -- -D warnings
   ```

3. Verify: all green, zero warnings.

4. Run: `git branch --show-current`
   Verify output matches the expected branch. Stop immediately if not.

5. Commit: `feat(chain): enrich MissingInputVariable with expected/provided keys`
