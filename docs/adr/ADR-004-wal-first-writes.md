# ADR-004: WAL-First Writes

**Date:** 2025
**Status:** Accepted

## Decision

The Write-Ahead Log (WAL) is always written and persisted before any mutation is applied to the MemTable. A write is never acknowledged to the client until the WAL entry is durable.

## Context

Writes are first buffered in memory (MemTable) before being flushed to disk (SSTable). If a node crashes before the MemTable is flushed, any buffered writes would be lost. Durability must be guaranteed regardless of when a crash occurs.

## Reasoning

Writing to the WAL first ensures that every acknowledged write can be recovered, even if the MemTable is lost in a crash. On restart, the node replays the WAL to reconstruct the MemTable state.

This is the standard approach used by LevelDB, RocksDB, Cassandra, and virtually every production LSM tree implementation. The performance cost of WAL writes is acceptable because WAL writes are sequential append operations — the fastest possible disk write pattern.

Acknowledging a write before WAL persistence (write-back caching) would improve throughput but risk data loss on crash. For a database, data loss is unacceptable.

## Consequences

- Every write incurs a sequential WAL write before acknowledgement
- WAL must be fsynced to guarantee durability — this is the main write latency cost
- WAL entries can be batched (group commit) to amortise fsync cost across multiple writes
- WAL grows indefinitely and must be periodically truncated after SSTable flushes
- WAL doubles as a replication log — other nodes can replay it to catch up
