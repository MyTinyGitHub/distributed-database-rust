# Homelab Database

A database engine built from scratch in Rust. Storage uses a B-tree index over a heap file; queries are routed via gRPC.

---

## Implementation Status

| Component | Status | Description |
|-----------|--------|-------------|
| `common` | ✅ Done | Shared types, errors, serialization, protobuf definitions |
| `wal` | ✅ Done | Write-Ahead Log with HMAC checksums, manifest management |
| `storage` | ✅ Done | gRPC partition node — heap file storage, B-tree index, manifest persistence |
| `query` | 🔨 In Progress | gRPC client stub; SQL parser/planner not yet implemented |
| `join` | 📋 Planned | Streaming join execution module |

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
                              │ gRPC
                              ▼
┌────────────┐  ┌────────────┐  ┌────────────┐
│ partition  │  │ partition  │  │ partition  │
│   node 1   │  │   node 2   │  │   node 3   │
│ B-tree idx │  │ B-tree idx │  │ B-tree idx │
│ Heap file  │  │ Heap file  │  │ Heap file  │
│ Manifest   │  │ Manifest   │  │ Manifest   │
└────────────┘  └────────────┘  └────────────┘
```

### Layers

- **query** — SQL parsing, lexing, logical query planning (in progress)
- **coordinator** — Physical planner, executor, catalog, join orchestration (planned)
- **storage** — Partition nodes: B-tree index → heap file, with manifest for durability

### Storage Engine

Each partition node exposes a gRPC service (`StorageEngineService`) and manages:

- **Heap file** — append-only row storage; each insert returns an `(offset, size)` pointer
- **B-tree index** — paged B-tree mapping index keys to heap file locations
- **Manifest** — JSON manifest persisted atomically (write-to-temp + rename) tracking tables and their indexes; loaded on startup to restore state across restarts

Supported operations: `CreateTable`, `DropTable`, `RegisterIndex`, `DropIndex`, `Write`, `ReadByIndex`.

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
├── storage/             # Partition node — gRPC server, B-tree index, heap file
│   └── src/
│       ├── btree/       # Paged B-tree (internal/leaf pages, page I/O)
│       ├── heap_file.rs # Append-only row store
│       ├── manifest.rs  # Table/index manifest with atomic saves
│       ├── record.rs    # On-disk record format
│       ├── table.rs     # Table: heap file + named B-tree indexes
│       ├── config.rs    # Directory configuration
│       └── main.rs      # gRPC server entry point
├── query/               # Query gateway (WIP — gRPC client stub only)
├── join/                # Join module (planned stub)
└── docs/
    └── adr/             # Architecture Decision Records
```

---

## Building

```bash
# Build all crates
cargo build --workspace

# Build specific crate
cargo build -p storage
cargo build -p wal

# Run tests
cargo test --workspace

# Check formatting and linting
cargo fmt --check
cargo clippy --workspace
```

### Running the storage node

```bash
cargo run -p storage
# Listens on [::1]:50052
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
- **Transport:** gRPC (tonic + prost)
- **Serialization:** bincode (row data), JSON (manifest), protobuf (wire protocol)
- **Crypto:** hmac-sha256 for WAL integrity
