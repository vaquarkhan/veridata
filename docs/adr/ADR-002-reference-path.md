# ADR-002: First Reference Integration Path

**Status:** Accepted  
**Date:** 2026-06-07

## Context

P2 requires exactly one real end-to-end source→sink path implemented through the connector SPI. The build spec defaults to Kafka → Apache Iceberg (streaming-native) as the moat path.

## Decision

Implement **Apache Kafka (source) → Apache Iceberg (sink)** as the v1 reference path.

- Source position kind: `KAFKA_OFFSET`
- Sink position kind: `ICEBERG_ROW` (snapshot + file + row coordinates)
- Default boundary mode for streaming: `OFFSET_RANGE` on source; sink scoped to matching ingest window

## Consequences

- `connectors/kafka/` and `connectors/iceberg/` are the P2 reference implementations
- P2 E2E tests use Testcontainers (Kafka + object store + Iceberg catalog)
- Pub/Sub → BigQuery deferred to P5 connector breadth

## Alternatives Considered

- **Pub/Sub → BigQuery:** More relatable for GCP users; deferred because Kafka→Iceberg better demonstrates streaming-native reconciliation with explicit offset boundaries.
