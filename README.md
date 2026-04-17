# Homelab Database

A database engine built from scratch in Rust. Storage uses a B-tree index over a heap file; queries are routed via gRPC.

---

## Implementation Status

| Component | Status | Description |
|-----------|--------|-------------|
| `common` | ✅ Done | Shared types, errors, serialization, protobuf definitions |
| `wal` | ✅ Done | Write-Ahead Log with HMAC checksums, manifest management |
| `storage` | ✅ Done | gRPC partition node — heap file storage, B-tree index, manifest persistence |
| `query` | 🔨 In Progress | gRPC test client for storage; SQL parser/planner not yet implemented |
| `join` | 📋 Planned | Stub only; streaming join execution not yet implemented |

---

## Architecture

Two independent gRPC services are currently running. The query layer and join module are not yet integrated.

```
┌─────────────────────────────────────────────────────────────┐
│              query (gRPC test client — WIP)                  │
│         drives storage directly; no SQL layer yet           │
└─────────────────────────────────────────────────────────────┘
                              │ gRPC
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              storage  [:50052]                               │
│         StorageEngineService (gRPC)                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Table                                               │   │
│  │  ├── B-tree index  (paged, key → heap pointer)      │   │
│  │  └── Heap file     (append-only row store)           │   │
│  └─────────────────────────────────────────────────────┘   │
│  Manifest  (JSON, atomic write-to-temp+rename)              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│              wal  [:50051]  (standalone, not yet integrated) │
│         WalService (gRPC)                                   │
│  ├── WalWriter  — appends HMAC-signed records to segment    │
│  ├── WalReader  — reads + verifies records from segments    │
│  └── Manifest   — tracks active segment per partition       │
└─────────────────────────────────────────────────────────────┘

┌──────────┐
│   join   │  (stub — not implemented)
└──────────┘
```

### Services

- **storage** `[:50052]` — The core storage engine. Exposes `StorageEngineService` over gRPC. Each table holds a heap file (append-only row store) and one or more named B-tree indexes mapping keys to heap offsets. A JSON manifest is persisted atomically on every schema change and reloaded on startup.

- **wal** `[:50051]` — Standalone write-ahead log service. Appends HMAC-SHA256-signed records to rolling segment files; a per-partition manifest tracks the active segment and the HMAC key. Not yet called by the storage engine — integration is a planned next step.

- **query** — Currently a gRPC client that drives the storage engine directly. SQL parsing and logical/physical planning are not yet implemented.

- **join** — Stub only.

### Storage Engine

`StorageEngineService` supports: `CreateTable`, `DropTable`, `RegisterIndex`, `DropIndex`, `Write`, `ReadByIndex`.

Each `Write` inserts a row into the heap file and updates every registered B-tree index for that table. `ReadByIndex` traverses the B-tree to find the heap offset and returns the raw row bytes.

### Key Design Decisions

- **Shared-nothing architecture** — Partition nodes are fully autonomous
- **Hash-based partitioning** — Prevents hotspots under uniform write load
- **WAL-first writes** — WAL persisted before any mutation is acknowledged (planned integration)
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
