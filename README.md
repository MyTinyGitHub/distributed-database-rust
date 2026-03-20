# Homelab Database

A distributed database engine built from scratch in Rust with an LSM tree storage engine.

---

## Implementation Status

| Component | Status | Description |
|-----------|--------|-------------|
| `common` | ✅ Done | Shared types, errors, serialization, protobuf definitions |
| `wal` | ✅ Done | Write-Ahead Log with HMAC checksums, manifest management |
| `storage` | 🔨 In Progress | Partition node, MemTable, SSTable, LSM tree |
| `query` | 🔨 In Progress | SQL parser, lexer, logical/physical planning |
| `join` | 🔨 In Progress | Streaming join execution module |

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                         query                                 │
│                   (SQL → logical plan)                       │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                       coordinator                             │
│          (physical planning → execution → catalog)          │
│                         join                                  │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────┐  ┌────────────┐  ┌────────────┐
│ partition  │  │ partition  │  │ partition  │
│   node 1   │  │   node 2   │  │   node 3   │
│ WAL        │  │ WAL        │  │ WAL        │
│ MemTable   │  │ MemTable   │  │ MemTable   │
│ SSTable    │  │ SSTable    │  │ SSTable    │
└────────────┘  └────────────┘  └────────────┘
```

### Layers

- **query** — SQL parsing, lexing, logical query planning
- **coordinator** — Physical planner, executor, catalog, join orchestration
- **storage** — Partition nodes: WAL → MemTable → SSTable (LSM tree)

### Key Design Decisions

- **Shared-nothing architecture** — Partition nodes are fully autonomous
- **Hash-based partitioning** — Prevents hotspots under uniform write load
- **WAL-first writes** — WAL persisted before any mutation is acknowledged
- **Fail fast** — Errors surface immediately, bounded retry with circuit breakers
- **Stateless join module** — Streams chunks, never buffers full datasets

See [ARCHITECTURE.md](./ARCHITECTURE.md) for full design documentation.

---

## Repository Structure

```
.
├── common/              # Shared types, errors, serialization, proto
├── wal/                 # Write-Ahead Log implementation
│   ├── src/
│   │   ├── config.rs    # Storage configuration
│   │   ├── errors.rs    # WAL error types
│   │   ├── lib.rs       # Public API
│   │   ├── manifest.rs  # WAL manifest (active segment, HMAC key)
│   │   └── wal.rs       # WAL reader/writer with HMAC checksums
│   └── data/            # WAL segment files
├── storage/             # Partition node (WIP)
├── query/               # Query gateway (WIP)
├── join/                # Join module (WIP)
└── docs/
    └── adr/             # Architecture Decision Records
```

---

## Building

```bash
# Build all crates
cargo build --workspace

# Build specific crate
cargo build -p wal
cargo build -p common

# Run tests
cargo test --workspace

# Check formatting and linting
cargo fmt --check
cargo clippy --workspace
```

---

## ADRs

| ID | Title |
|----|-------|
| ADR-001 | Shared-nothing architecture |
| ADR-002 | Hash-based partitioning |
| ADR-003 | Logical/physical planning split |
| ADR-004 | WAL-first writes |
| ADR-005 | Stateless join module |
| ADR-006 | Fail fast with circuit breakers |

---

## Tech Stack

- **Language:** Rust
- **Serialization:** bincode, protobuf (tonic-build)
- **Crypto:** hmac-sha256 for WAL integrity
