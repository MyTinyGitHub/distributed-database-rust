# Contributing

Personal development guidelines for this monorepo. These rules exist to keep the architecture clean as the codebase grows. Future me will thank present me for following them.

---

## Layer Boundary Rules

These are non-negotiable. The architecture only works if boundaries are respected.

**gateway** imports from: nothing in this monorepo (only std and external crates)
**coordinator** imports from: `gateway` types (AST, logical plan) only
**storage-engine** imports from: nothing in this monorepo (only std and external crates)
**coordinator** communicates with **storage-engine** via network/IPC — never direct crate imports

If a change requires crossing a layer boundary, the design needs to change — not the rule.

---

## Before Starting Work

- Know which layer the change belongs to
- If it touches multiple layers, it probably needs to be split into separate commits
- Check `ARCHITECTURE.md` if unsure where something belongs

---

## Code Standards

### Rust
- No `.unwrap()` or `.expect()` outside of tests — use `?` and proper error types
- No silent failures — errors always propagate explicitly
- Prefer borrowing over cloning in hot paths
- Iterators over manual loops where it reads naturally
- Every public function, struct, and enum gets a `///` doc comment
- Module-level `//!` comment in every `mod.rs` and `lib.rs`

### Error Handling
- Each layer has its own error type — never reuse error types across layer boundaries
- Error messages are descriptive and include enough context to debug without a stack trace
- Use `thiserror` for defining error types

### Testing
- Every non-trivial function has at least one unit test
- Edge cases are always tested — empty input, boundary values, failure paths
- Property-based tests with `proptest` for any serialization, hashing, or data transformation
- Integration tests for cross-layer interactions
- Run tests before every commit: `just test`

---

## Commit Guidelines

- One logical change per commit
- Commit message format: `[component] short description`
  - `[storage] add WAL write before memtable insert`
  - `[coordinator] implement chunked streaming for join module`
  - `[gateway] add SELECT with WHERE clause parsing`
- If a commit message needs "and" it should probably be two commits

---

## Task Runner

This monorepo uses `just` as the task runner. Common commands:

```bash
just test              # run all tests
just test-gateway      # run gateway tests only
just test-coordinator  # run coordinator tests only
just test-storage      # run storage engine tests only
just lint              # run clippy
just fmt               # run rustfmt
just doc               # generate and open docs
just check             # fmt + lint + test in one go
```

---

## Adding a New Component

1. Create a new folder at the monorepo root
2. Add a `README.md` explaining what it does and what it does NOT do
3. Add a `//!` module comment to `lib.rs` or `main.rs`
4. Add it to the context table in `.opencode/agents/AGENT.md`
5. Update `ARCHITECTURE.md` with the new component
6. Add a `just test-[component]` task

---

## AI Agents

Agents live in `.opencode/agents/`. Use them via `@agent-name` in OpenCode.

| Agent | Use for |
|---|---|
| `@AGENT` | Code review |
| `@AGENT-ARCHITECTURE` | Checking layer boundary violations |
| `@AGENT-TEST` | Generating tests for new code |
| `@AGENT-DOCS` | Generating doc comments and READMEs |
| `@AGENT-DEVIL` | Stress testing a design decision |
| `@AGENT-PERFORMANCE` | Performance review and optimisation |

---

## Sync to GitHub

This monorepo is hosted on a private Gitea instance and synced to public GitHub automatically via `just sync`. The sync pushes to the public repo so the GitHub profile stays up to date.

Make sure any work intended to be public is in a clean, documented state before syncing.
