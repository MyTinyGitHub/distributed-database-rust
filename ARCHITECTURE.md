# Architecture

This document describes the architecture of the distributed database engine. It captures the key design decisions, layer boundaries, and invariants that must be preserved as the system evolves.

---

## Overview

A distributed database engine built from scratch in Rust. The system accepts SQL queries, plans and executes them across distributed partition nodes, and stores data using an LSM tree storage engine.

The core design philosophy is **simplicity through strict boundaries**. Each layer does one thing, knows only about the layer directly below it, and has no knowledge of implementation details outside its own scope.

---

## Repository Structure

```
monorepo/
├── gateway/              # SQL parser, lexer, CLI interface
├── coordinator/          # Executor, planners, catalog, join module
├── storage-engine/       # Partition node, WAL, MemTable, LSM Tree
├── interpreter/          # Custom language interpreter (Monkey lang in Rust)
└── .opencode/
    └── agents/           # AI agent definitions
        ├── AGENT.md
        ├── AGENT-ARCHITECTURE.md
        ├── AGENT-TEST.md
        ├── AGENT-DOCS.md
        ├── AGENT-DEVIL.md
        └── AGENT-PERFORMANCE.md
```

---

## System Architecture

```
Client / CLI
     │
     ▼
┌─────────────┐
│   gateway   │  SQL parsing, lexing, logical planning
└─────────────┘
     │ logical query plan
     ▼
┌─────────────────────────────────────────┐
│              coordinator                │
│  physical planner → executor → catalog  │
│              join module                │
└─────────────────────────────────────────┘
     │ partition queries
     ▼
┌────────────┐  ┌────────────┐  ┌────────────┐
│ partition  │  │ partition  │  │ partition  │
│   node 1   │  │   node 2   │  │   node 3   │
│ WAL        │  │ WAL        │  │ WAL        │
│ MemTable   │  │ MemTable   │  │ MemTable   │
│ SSTable    │  │ SSTable    │  │ SSTable    │
└────────────┘  └────────────┘  └────────────┘
```

---

## Layers

### Gateway

The entry point to the system. Accepts SQL queries via CLI or network interface.

**Responsibilities:**
- Lexing and parsing SQL into an AST
- Building a logical query plan
- Validating queries against catalog metadata (table existence, column types, cardinality)

**Strict boundaries — gateway never:**
- References coordinator or storage-engine internals
- Performs physical planning or execution
- Knows about partition topology, node addresses, or storage formats

---

### Coordinator

Orchestrates query execution across partition nodes.

**Responsibilities:**
- Translating logical plans into physical execution plans
- Fanning out queries to the relevant partition nodes
- Owning the table catalog — schema, statistics, partition routing
- Executing joins via the stateless join module
- Caching query plans using DP-based optimisation
- Handling partition node failures — fail fast, bounded retry

**Strict boundaries — coordinator never:**
- Knows about SSTable, WAL, or MemTable internals
- Bypasses the catalog for metadata
- Buffers full datasets for joins — always streams chunks
- Returns partial results silently on partition failure

---

### Storage Engine

Fully autonomous partition nodes. Each node owns its data completely.

**Responsibilities:**
- Accepting read and write requests for its partition
- Writing to WAL before any mutation
- Buffering writes in MemTable, flushing to SSTable in batches
- Background compaction of SSTables
- Serving reads from MemTable and SSTable

**Strict boundaries — storage-engine never:**
- References coordinator, catalog, or join module
- Coordinates directly with other partition nodes
- Acknowledges a write before WAL is persisted

---

## Key Design Decisions

### Shared-Nothing Architecture
Each partition node is fully autonomous. No shared memory, no shared storage between nodes. Enables independent scaling and failure isolation.

### Hash-Based Partitioning
Data is distributed by hash of the partition key. Chosen over range-based partitioning to prevent hotspots under uniform write load.

### Logical / Physical Planning Split
The logical planner works purely with abstract catalog metadata — it has no knowledge of physical storage. The physical planner translates the logical plan into concrete execution steps. This means the logical planner is completely storage-agnostic and can be tested independently.

### Stateless Join Module
The join module lives inside the coordinator and is completely stateless. It receives datasets and a join strategy decided by the planner, processes data in chunks, and streams results. It never loads full datasets into memory.

### DP-Based Join Optimisation with Plan Caching
The planner uses dynamic programming to find the optimal join order across multiple tables. Plans are cached and invalidated by the catalog when table statistics change significantly.

### WAL-First Writes
The WAL is always written before any mutation is applied to the MemTable. A write is never acknowledged until the WAL entry is durable. This ensures recovery is always possible from the WAL alone.

### Fail Fast with Circuit Breakers
Errors surface immediately. No silent failures, no indefinite retries. Bounded retry logic with circuit breakers on partition node communication. When a node is unresponsive, the circuit opens and requests fail fast.

### Minimum Two Nodes
At least two partition nodes exist at all times. When one node goes down the other can serve requests, providing a baseline level of availability without complex consensus protocols.

### Namespace as Pure Router
The coordinator's catalog acts as a pure routing layer — it knows partition topology but not storage internals. It is stateless enough to scale horizontally. Consistent hashing keeps routing consistent as namespace instances are added.

---

## Data Flow

### Write Path
```
Client → Gateway (parse) → Coordinator (route) → Partition Node
                                                  → WAL (persist first)
                                                  → MemTable (buffer)
                                                  → SSTable (flush in batch)
```

### Read Path
```
Client → Gateway (parse + logical plan)
       → Coordinator (physical plan + catalog lookup)
       → Fan out to relevant partition nodes
       → Partition nodes read from MemTable + SSTable
       → Results streamed back to coordinator
       → Join module assembles if needed (chunked streaming)
       → Results returned to client
```

### Compaction (background)
```
Partition Node → periodic SSTable consolidation
              → older SSTables merged into larger ones
              → never blocks reads or writes
```

---

## Failure Handling

| Scenario | Behaviour |
|---|---|
| Partition node goes down | Fail fast, circuit breaker opens, retry on second node |
| WAL write fails | Write rejected, MemTable not modified |
| Partial partition failure mid-query | Error returned, no partial results |
| Node restart | WAL replayed to recover MemTable state |
| Catalog corruption | WAL for catalog replayed to reconstruct state |

---

## Interpreter

A separate project in the same monorepo. An interpreter for the Monkey language, built in Rust following "Writing an Interpreter in Go". Serves as both a learning project and the foundation for the database query language frontend.

### Pipeline
```
Source code → Lexer → Tokens → Parser → AST → Evaluator → Result
```

### Boundaries
- Each stage only knows about its own input and output types
- AST is immutable after construction
- Each stage has its own distinct error type
- Evaluator never modifies AST nodes

---

## Invariants

These must never be violated:

1. WAL is always written before any MemTable mutation
2. Layer boundaries are never crossed — each layer imports only from the layer directly below
3. Partition nodes never coordinate directly with each other
4. Join module never buffers full datasets — always streams chunks
5. Catalog is the single source of truth for partition topology and table statistics
6. Plan cache is always invalidated when catalog statistics change
7. No write is acknowledged before WAL is durable
8. Partial query results are never returned silently
