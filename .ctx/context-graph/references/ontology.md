# Ontology — Entity Type Normalization

## Entity Types

| Canonical Type | Aliases / Examples |
|---------------|-------------------|
| `trait` | Rust trait, interface, abstract type |
| `struct` | Rust struct, data type, model |
| `module` | Rust module, crate, package |
| `function` | method, fn, associated function |
| `pattern` | design pattern, architectural pattern |
| `concept` | idea, abstraction, domain concept |
| `tool` | CLI tool, library, dependency |
| `feature` | cargo feature flag, optional capability |
| `backend` | LLM provider, vector store backend, embedding backend |
| `file` | source file, config file |
| `issue` | bug, problem, error |
| `decision` | architectural decision, trade-off |

## Relation Types

| Canonical Type | Aliases |
|---------------|---------|
| `implements` | impl, realizes |
| `depends_on` | uses, requires, imports |
| `contains` | has, owns, includes |
| `extends` | derives, inherits |
| `causes` | leads_to, triggers |
| `configures` | enables, gates |
| `replaces` | supersedes, deprecates |
| `related_to` | associated_with (weak link) |

## Normalization Rules

- Always use the canonical type from the tables above.
- If unsure, use `concept` for entities and `related_to` for relations.
- Compound entities ("OpenAI embedding backend") should be a single node
  with type `backend`, not split into multiple nodes.
