# ADR-006: Fail Fast with Circuit Breakers

**Date:** 2025
**Status:** Accepted

## Decision

Errors surface immediately. No silent failures. Bounded retry logic with circuit breakers on partition node communication. When a node is unresponsive the circuit opens and subsequent requests fail fast without waiting for a timeout.

## Context

In a distributed system, nodes can be slow or unresponsive. The system needs a strategy for handling these failures. Options range from aggressive retry (keep trying until success) to fail fast (surface the error immediately).

## Reasoning

Silent failures are the most dangerous failure mode in a database. A query that returns partial results without indicating failure is worse than a query that returns an explicit error. Users can handle errors — they cannot handle silently wrong data.

Fail fast was chosen because:
- Errors are explicit and diagnosable
- The caller (client) can decide whether to retry, not the database
- Timeouts and retries inside the database hide problems rather than surfacing them
- At minimum 2 nodes per partition, a fast failure allows immediate rerouting to the healthy node

Circuit breakers prevent cascading failures. If a node is unresponsive, opening the circuit avoids hammering it with requests while it recovers, and prevents threads from piling up waiting for timeouts.

## Consequences

- Clients must handle explicit errors and implement their own retry logic if needed
- Circuit breaker state must be tracked per partition node
- Circuit breaker thresholds (failure count, timeout) need tuning
- Minimum 2 nodes per partition is required for this strategy to provide availability
- WAL ensures that failed writes can be retried safely — they are idempotent if the WAL entry was not applied
