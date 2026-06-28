# Contributing

Personal development guidelines for this monorepo. These rules exist to keep the architecture clean as the codebase grows. Future me will thank present me for following them.

---

## Layer Boundary Rules

These are non-negotiable. The architecture only works if boundaries are respected.

- **common**: Contains shared protobuf definitions and basic structs. Does not import from other monorepo crates.
- **wal**: The Write-Ahead Log microservice. Does not import from `storage`, `query`, or `join`.
- **storage**: The storage engine microservice. Communicates with `wal` via gRPC client — never direct crate imports.
- **query / join**: Frontend and coordinator components. Communicate with `storage` via gRPC client — never direct crate imports.

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
- Run tests before every commit: `cargo test`

---

## Commit Guidelines

- One logical change per commit
- Commit message format: `[component] short description`
- Examples:
  - `[storage] add WAL write before B+Tree page insert`
  - `[join] implement coordinator placeholder stub`
  - `[wal] implement checksum validation loop`
- If a commit message needs "and" it should probably be two commits

---

## Cargo Commands

This monorepo uses standard cargo commands for workspace development:

```bash
cargo check             # check code compilation
cargo test              # run all unit and integration tests
cargo test -p storage   # run storage engine tests only
cargo clippy            # run lints
cargo fmt               # format code
```

---

## Adding a New Component

1. Create a new folder at the monorepo root
2. Add a `README.md` explaining what it does and what it does NOT do
3. Add a `//!` module comment to `lib.rs` or `main.rs`
4. Update `ARCHITECTURE.md` with the new component

---

## Sync to GitHub

This monorepo is hosted on a private Gitea instance and synced to public GitHub automatically. The sync pushes to the public repo so the GitHub profile stays up to date.

Make sure any work intended to be public is in a clean, documented state before syncing.

