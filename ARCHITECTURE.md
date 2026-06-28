# Architecture

This document describes the architecture of the distributed database engine. It captures the key design decisions, layer boundaries, and invariants that must be preserved as the system evolves.

---

## Overview

A distributed database engine built from scratch in Rust. The system accepts query requests, plans/executes them across partition nodes, and stores data using a paged B+Tree index mapping to binary heap record files.

The core design philosophy is **simplicity through strict boundaries**. Each layer does one thing, knows only about the layer directly below it, and has no knowledge of implementation details outside its own scope.

---

## Repository Structure

```
monorepo/
├── common/               # Shared protobuf definitions and structs
├── wal/                  # Write-Ahead Log gRPC server
├── storage/              # B+Tree & Heap File storage engine gRPC server
├── query/                # Client queries and integration tests
└── join/                 # Coordinator placeholder / join stub
```

---

## System Architecture

```
Client / Query Test
     │
     ▼
┌─────────────┐
│    query    │  Client executor and gRPC query driver
└─────────────┘
     │ gRPC requests
     ▼
┌─────────────┐  gRPC Writes   ┌────────────┐
│   storage   │ ─────────────> │    wal     │
│   service   │                │  service   │
│  (B+Tree &  │                │  (Write-   │
│   Heaps)    │                │   Ahead)   │
└─────────────┘                └────────────┘
```

---

## Layers

### Client / Query Layer

The frontend interface. Submits read/write requests to the partition nodes using the gRPC interface.

**Strict boundaries — query layer never:**
- Directly accesses storage-engine or WAL file internals.
- Performs page manipulation or logs mutations directly.

---

### Storage Engine

Autonomous partition nodes using B+Tree indexing on binary Heap Files.

**Responsibilities:**
- Accepting read and write requests for its partition.
- Writing to the Write-Ahead Log (WAL) service before applying any database modifications.
- Appending record payloads sequentially to Heap Files (`heap.db`).
- Keeping track of key-to-record-offset mapping using disk-based B+Tree pages (`.idx` files).
- Reading index keys and record structures directly from disk files.

**Strict boundaries — storage-engine never:**
- Coordinates directly with other partition nodes.
- Acknowledges a write before the WAL entry is flushed and durable.
- Cross-references query coordination details or global schemas.

---

### WAL Service

Provides write-ahead logging to guarantee transaction durability and crash-recovery capabilities.

**Responsibilities:**
- Appending incoming logs sequentially.
- Serving historical WAL entries for replay and state recovery.
- Re-playing and validating logs (using HMAC checksum verification).

**Strict boundaries — WAL service never:**
- Understands B+Tree page structure or heap data payload semantics.
- Directly accesses storage databases or index files.

---

## Key Design Decisions

### Shared-Nothing Architecture
Each partition node is fully autonomous. No shared memory, no shared storage between nodes. Enables independent scaling and failure isolation.

### B+Tree Page Storage
Instead of buffering and sorting writes dynamically in memory, writes are indexed on disk page files (`.idx`) organized as a B+Tree structure. Each node has a fixed size (4096 bytes) and points to child pages or to records in heap database files.

### Sequential Heap Files
Record data payloads are written sequentially to a heap file (`heap.db`). The offset and length of each record are stored in the B+Tree indexes, ensuring fast O(log N) lookup and appending.

### WAL-First Writes
The WAL is always written before any page mutation or heap insertion is done. A write is never acknowledged until the WAL entry is durable on disk. This ensures recovery is always possible from the WAL alone.

### Fail Fast
Errors surface immediately. No silent failures, no indefinite retries. When communication with the WAL service fails, the storage write is rejected immediately.

---

## Data Flow

### Write Path
```
Client Request → Storage Node (write)
               → WAL Service (append & sync to log)
               → Heap File (append record payload)
               → B+Tree Index (traverse B+Tree pages and insert key mapping)
               → Acknowledge success to Client
```

### Read Path
```
Client Request → Storage Node (read)
               → B+Tree Index (traverse index pages to find heap offset & size)
               → Heap File (read buffer at offset)
               → Return record payload to Client
```

---

## Failure Handling

| Scenario | Behaviour |
|---|---|
| WAL service goes down | Write rejected, storage node returns error |
| Partial write failure | Transaction failed, disk pages remain unmodified |
| Node restart / Crash | WAL replayed to reconstruct consistent B+Tree and heap state |

---

## Invariants

These must never be violated:

1. WAL is always written and synced before any B+Tree index page or heap file mutation.
2. Layer boundaries are respected — the storage engine accesses the WAL service only via the gRPC client interface.
3. Partition nodes never coordinate directly with each other.
4. No write is acknowledged before the WAL is durable.
5. Partial query results are never returned silently.
