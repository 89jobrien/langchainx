# Developer Setup

## Prerequisites

- Rust (stable toolchain)
- [`rust-script`](https://rust-script.org/) — used for secret resolution
- [`op`](https://developer.1password.com/docs/cli/) — 1Password CLI (optional but recommended)
- Nushell (for `.nuenv` auto-sourcing)

## Environment Variables

Secrets and configuration are resolved at shell entry via `.nuenv`, which is sourced
automatically by nushell when you enter the project directory.

### How it works

`.nuenv` runs `scripts/resolve-env.rs` (a `rust-script` script) which resolves each variable
in priority order:

1. **Already set** in your environment — never overwritten
2. **1Password plugin** (`op plugin run`) — for services with a configured op plugin (e.g. OpenAI)
3. **1Password vault** (`op read`) — for custom vault items
4. **Default** — hardcoded sensible defaults (local service URLs, etc.)

The script prints `KEY=value` lines which `.nuenv` loads into the shell environment.

### Setup

1. Copy the example env file:

   ```sh
   cp .nuenv.example .nuenv
   ```

2. Update `scripts/resolve-env.rs` with your 1Password vault item names/paths. The
   `op://Personal/<item>/<field>` refs must match your vault.

3. For services with a configured op plugin, ensure the plugin is set up:

   ```sh
   op plugin init openai   # example
   ```

4. Enter the project directory — `.nuenv` is sourced automatically. To reload manually:

   ```sh
   source .nuenv
   ```

### Adding a new variable

Add a row to the `vars` table in `scripts/resolve-env.rs`:

```rust
Var { name: "MY_API_KEY", source: Source::Op("op://Personal/<item>/credential") },
```

Source variants:

| Variant | When to use |
|---|---|
| `Source::Plugin { plugin, env_var }` | Service has a configured `op plugin` |
| `Source::Op(ref)` | Secret lives in a 1Password vault item |
| `Source::Default(value)` | Non-secret config with a sensible default |

### Without 1Password

If `op` is not available, all `Source::Op` and `Source::Plugin` vars resolve to empty strings.
Set them manually before running:

```sh
$env.OPENAI_API_KEY = "sk-..."
```

Or export them from your shell profile.

## Running tests

```sh
# Unit tests (no external services required)
cargo test --all-features

# Integration tests (require live services)
cargo test --all-features --test e2e_local_llm   # requires Ollama running
```

## Feature flags

Most integrations are opt-in via Cargo feature flags. Key flags:

| Flag | Enables |
|---|---|
| `postgres` | pgvector vectorstore + SqlDatabase chain |
| `qdrant` | Qdrant vectorstore |
| `surrealdb` | SurrealDB vectorstore |
| `ollama` | Ollama LLM + embeddings |
| `fastembed` | Local FastEmbed embeddings |
| `mistralai` | MistralAI LLM + embeddings |
| `git` | Git commit document loader |
| `lopdf` / `pdf-extract` | PDF document loaders |
| `html-to-markdown` | HTML→Markdown loader |
| `tree-sitter` | Source code loader |
| `rss` | RSS feed loader |
| `sitemap` | Sitemap loader |

Build with all features (matches CI):

```sh
cargo build --all-features
```
