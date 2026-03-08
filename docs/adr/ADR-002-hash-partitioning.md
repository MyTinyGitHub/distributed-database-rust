# ADR-002: Hash-Based Partitioning

**Date:** 2025
**Status:** Accepted

## Decision

Data is distributed across partition nodes using a hash of the partition key.

## Context

Data needs to be distributed across partition nodes. The two main strategies are range-based partitioning (consecutive keys go to the same node) and hash-based partitioning (keys are hashed to determine their node).

## Reasoning

Hash-based partitioning was chosen to prevent hotspots. With range-based partitioning, sequential writes (e.g. time-series data, auto-incrementing IDs) all land on the same partition node, creating a write hotspot. Hash-based partitioning distributes writes uniformly across nodes regardless of key pattern.

The tradeoff is that range queries require scanning all partitions rather than a contiguous range of nodes. For this use case, uniform write distribution is more important than range query locality.

## Consequences

- Write load is distributed evenly across partition nodes
- Range queries fan out to all partitions — acceptable tradeoff
- Consistent hashing should be used when adding/removing nodes to minimise data movement
- The partition key must be chosen carefully by the schema designer — a poor partition key still causes hotspots
