# ADR-004: WAL-First Writes

**Date:** 2025
**Status:** Accepted

## Decision

The Write-Ahead Log (WAL) is always written and persisted before any mutation is applied to the database's heap files and B+Tree indexes. A write is never acknowledged to the client until the WAL entry is durable.

## Context

Writes are applied to B+Tree indexes (stored as paged files) and heap files on disk. If a node crashes during a write or before buffers are flushed, database structures can become corrupt or inconsistent. Durability and consistency must be guaranteed regardless of when a crash occurs.

## Reasoning

Writing to the WAL first ensures that every acknowledged write can be recovered or reconstructed, even if the database index page files or heap files are in an inconsistent state or lost in a crash. On restart, the node can replay the WAL to reconstruct or reconcile the state of B+Tree indexes and heap records.

This is the standard approach used by SQLite, PostgreSQL, and virtually every production B+Tree implementation (referred to as ARIES-style recovery or WAL-first writing). The performance cost of WAL writes is acceptable because WAL writes are sequential append operations — the fastest possible disk write pattern.

Acknowledging a write before WAL persistence (write-back caching) would improve throughput but risk data loss on crash. For a database, data loss is unacceptable.

## Consequences

- Every write incurs a sequential WAL write before acknowledgement
- WAL must be fsynced to guarantee durability — this is the main write latency cost
- WAL entries can be batched (group commit) to amortise fsync cost across multiple writes
- WAL grows indefinitely and must be periodically truncated after checkpointing or backup of the heap/index files
- WAL doubles as a replication log — other nodes can replay it to catch up

