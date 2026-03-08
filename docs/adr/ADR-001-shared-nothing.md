# ADR-001: Shared-Nothing Architecture

**Date:** 2025
**Status:** Accepted

## Decision

Partition nodes are fully autonomous. No shared memory, no shared storage between nodes.

## Context

The database needs to scale horizontally across multiple servers. Two broad approaches exist: shared-disk (nodes share a storage layer) and shared-nothing (each node owns its data completely).

## Reasoning

Shared-nothing was chosen because:
- Failure isolation is clean — one node failing does not affect others
- Scaling is straightforward — add nodes, rebalance partitions
- No contention on a shared storage layer
- Each partition node can be reasoned about independently
- Simpler to implement correctly at this stage

Shared-disk would require a distributed lock manager and a shared storage layer, adding significant complexity without clear benefit at this scale.

## Consequences

- Joins across partitions require data to be shipped to the coordinator — this is handled by the streaming join module
- Rebalancing partitions when adding nodes requires data movement
- Each node must be independently durable — WAL and compaction per node
