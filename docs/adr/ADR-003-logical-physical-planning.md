# ADR-003: Logical / Physical Planning Split

**Date:** 2025
**Status:** Accepted

## Decision

Query planning is split into two distinct stages: a logical planner that works with abstract catalog metadata, and a physical planner that translates logical plans into concrete execution steps.

## Context

Query planning needs to validate queries, optimise join order, and produce an execution plan. This can be done in a single pass or split into logical and physical stages.

## Reasoning

Splitting logical and physical planning provides clean separation of concerns:

- The **logical planner** only needs catalog metadata — table existence, column types, cardinality estimates, join selectivity. It has zero knowledge of physical storage, partition topology, or node addresses. It can be tested completely independently of the storage layer.

- The **physical planner** takes the logical plan and maps it to concrete operations — which partitions to scan, which indexes to use, which join strategy to apply. It knows about physical layout but does not re-validate the query.

This separation means the logical planner is storage-agnostic. The storage engine can change underneath without touching the logical planner. It also makes the system easier to reason about — each planner stage has a clear, limited responsibility.

## Consequences

- Two planning stages add a small overhead per query — negligible compared to I/O cost
- The logical plan is an intermediate representation that must be defined as a shared type
- Physical planner decisions depend on catalog statistics being accurate and up to date
- Plan caching operates on physical plans — cache invalidation is triggered by catalog changes
