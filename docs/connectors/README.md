# Connectors (P2)

Reference path: **Kafka → Apache Iceberg** via the [SPI](../spi/).

## Crates

| Crate | Role |
|-------|------|
| `veridata-spi` | `SourceConnector`, `SinkConnector`, boundary parsing, schema drift |
| `veridata-connector-kafka` | Source: `OFFSET_RANGE` boundaries |
| `veridata-connector-iceberg` | Sink: Parquet warehouse + snapshot manifest, pushdown hashing |
| `veridata-e2e` | Fault-injected E2E tests through SPI |

## Run E2E tests

```bash
cargo test -p veridata-e2e
```

## Kafka backend

Default build uses an in-memory Kafka source (reproducible offset-range reads). Enable `rdkafka-backend` on Linux CI for live Testcontainers (optional):

```bash
cargo test -p veridata-connector-kafka --features rdkafka-backend
```

## Iceberg warehouse layout

```
warehouse/orders/
  metadata/snapshots.json   # [{ "snapshot_id": 1, "files": ["data/snap-1.parquet"] }]
  data/snap-1.parquet
```

Sink boundaries use `BATCH_ID` with JSON `{"snapshot_from":1,"snapshot_to":1}`.
