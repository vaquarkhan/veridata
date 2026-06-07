# Roadmap — not yet implemented

P0–P3 in-repo bugs and acceptance gaps are addressed on `main`.
The items below are **outstanding** and require new connectors, SDK integrations, or release infrastructure.

## P4 — distribution and adoption

| Item | Status | Notes |
|------|--------|-------|
| **crates.io publish** | Not done | `veridata-core`, `veridata-proof`, `veridata-cli` metadata + release workflow |
| **GitHub binary releases** | Not done | Multi-OS static `veridata` binaries (no Docker required) |
| **Optional Docker image** | Not done | Secondary channel only |
| **PyPI / Python bindings** | Not done | Would need PyO3 wheel for data-engineer adoption |

## Cloud connectors (design only today)

No `s3`, `gcs`, `azure`, or cloud `kms` SDK usage exists in source. Only in-memory Kafka + filesystem Iceberg are implemented.

| Target | Source | Sink | Proof store | Signing |
|--------|--------|------|-------------|---------|
| **AWS** | MSK (rdkafka) | Iceberg-on-S3 | S3 | AWS KMS |
| **GCP** | Pub/Sub | BigQuery (SQL pushdown) | GCS | Cloud KMS |
| **Azure** | Event Hubs | Delta-on-ADLS | ADLS | Key Vault |
| **Databricks** | — | Delta / Unity Catalog | S3/ADLS | Databricks secrets or cloud KMS |

**Databricks** is the highest-value target for most organizations and is completely absent today.

## Real KMS SDKs (AC-C7 production)

Current implementation on `main`: **file-backed** `FileKmsSigner` and `PubkeyDirectory` for local/on-prem stand-in.

Still needed:

- AWS KMS (`Sign`, key rotation, historical pubkey lookup)
- GCP Cloud KMS
- Azure Key Vault
- Envelope signing with audit-friendly key ids in VRP `producer` metadata

## Warehouse SQL pushdown (AC-D4 production)

Filesystem Iceberg connector uses **Parquet column projection** client-side. That is not warehouse SQL pushdown.

Still needed:

- BigQuery: `SELECT … SHA256(…)` so rows never leave the warehouse
- Snowflake / Databricks: Spark SQL hashing pushdown
- Honest `pushdown_used` only when hashing runs in the warehouse engine

## Pipeline integration (not built — common oversell)

These are **not** on `main`. Veridata v0.1 **detects and proves** faults; it does not operate the pipeline after a FAIL.

| Integration | Status | Notes |
|-------------|--------|-------|
| Inline verification gate before promote | DIY | Call `veridata verify --check` in CI/CD |
| Dead Letter Queue (DLQ) routing | Not done | Route missing/mutated evidence to your DLQ topic |
| Idempotent replay / backfill | Not done | Orchestrator (Airflow, Flink, custom) |
| Auto-remediation agent | Not done | Distinct from AIOps/IaC products |
| Proof store on S3/GCS/ADLS | Not done | Local filesystem today |

See [POSITIONING.md](../POSITIONING.md) for accurate enterprise messaging.

## P5 — breadth (post-1.0)

Per `CURSOR-BUILD-SPEC.md` phase P5; not started:

| Feature | ID | Status |
|---------|-----|--------|
| Multi-hop localization | F-B9 | Not done |
| Transform-aware reconciliation | F-B10 | Not done |
| Transparency-log anchoring | F-C8 | Not done |
| Approximate pre-filter | F-B7 | Not done |
| Additional connectors (Pub/Sub→BigQuery, etc.) | F-D5 | Not done |
| Continuous / streaming mode | — | Not done |
| Late-arrival **supersede chain** (full F-B8) | F-B8 | Partial — window parse + verify gate only; no chain replay |
| Full commitment recompute without `fp` in proof | — | Partial — arithmetic + root consistency; cannot rebuild matched Merkle without evidence |

## Suggested implementation order

1. GitHub Releases + crates.io (adoption without Docker)
2. Databricks/Delta connector + S3 proof store
3. AWS KMS + MSK hardening
4. BigQuery SQL pushdown path
5. P5 features per adopters
