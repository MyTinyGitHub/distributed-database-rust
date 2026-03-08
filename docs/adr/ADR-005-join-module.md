# ADR-005: Join Module as Stateless Coordinator Module

**Date:** 2025
**Status:** Accepted

## Decision

The join module lives inside the coordinator as a stateless module. It is not a separate service. It receives datasets and a join strategy decided by the planner, processes data in chunks, and streams results. It never loads full datasets into memory.

## Context

Joins across partitions require data from multiple nodes to be combined. This logic needs to live somewhere in the system. Options considered: separate join service, part of the executor, or a dedicated module inside the coordinator.

## Reasoning

A separate join service was considered but rejected at this stage. The overhead of a separate service (deployment, network hops, serialization) is not justified when the join module has no state to scale independently. It is a pure function: datasets in, joined dataset out.

Living inside the coordinator keeps the deployment simple while maintaining clean internal separation. The join module is isolated enough internally that it could be extracted to a separate service later if scale demands it.

Streaming chunk-by-chunk rather than loading full datasets is non-negotiable for large joins. A join that requires two 10GB datasets to be fully loaded into coordinator memory would make the coordinator a bottleneck and limit practical query size.

Join strategy (hash join, merge join, nested loop) is decided by the planner based on dataset size and sort order. The join module just executes the strategy it is given.

## Consequences

- Join module is a coordinator-internal module, not a separate binary
- All joins stream data in configurable chunk sizes
- Sorted inputs use merge join (constant memory), unsorted inputs use hash join (memory proportional to smaller dataset)
- Unoptimised joins (no index, no sort order) are slow by design — this is a schema design problem, not a database problem
- Extracting to a separate service later is possible without changing the interface
