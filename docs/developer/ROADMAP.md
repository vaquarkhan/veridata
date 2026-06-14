# Roadmap — P4 cloud layer shipped on `main`

P0–P3 and P4 cloud connectors/KMS are implemented. Remaining P4 items are distribution (crates.io, GitHub Releases).

## P4 — distribution and adoption

| Item | Status | Notes |
|------|--------|-------|
| **Dependabot** | Enabled | Weekly PRs for Cargo, `python/` pip, and GitHub Actions |
| **PyPI (`veridata-vrp`)** | Package ready | VRP offline verifier; name avoids conflict with [VeriData](https://pypi.org/project/VeriData/) pandas package |
| **crates.io publish** | Not done | `veridata-core`, `veridata-proof`, `veridata-cli` metadata + release workflow |
| **GitHub binary releases** | Not done | Multi-OS static `veridata` binaries (no Docker required) |
| **Optional Docker image** | Not done | Secondary channel only |

## Cloud connectors — **implemented** (`veridata-connector-cloud`)

Build with `cargo build -p veridata-cli --features cloud`. Examples: [CLOUD-EXAMPLES.md](../connectors/CLOUD-EXAMPLES.md).

| Target | Source | Sink | Proof store | Signing |
|--------|--------|------|-------------|---------|
| **AWS** | `msk` (MSK IAM via rdkafka) | `iceberg_s3` (Parquet on S3) | `store.kind: s3` | `crypto.kms_provider: aws` |
| **GCP** | `pubsub` | `bigquery` (SQL `SHA256` pushdown) | `store.kind: gcs` | `crypto.kms_provider: gcp` |
| **Azure** | `eventhubs` | Delta via `iceberg_s3`/ADLS | `store.kind: adls` | `crypto.kms_provider: azure` |
| **Databricks** | — | `databricks` / `databricks_delta` | S3/ADLS | Cloud KMS or file |

Demo path unchanged: `memory_kafka` → filesystem `iceberg`.

## Cloud KMS SDKs (AC-C7 production) — **implemented** (`veridata-cloud`)

| Provider | Crate feature | Config |
|----------|---------------|--------|
| **File** (default) | — | `kms_provider: file` + key files |
| **AWS KMS** | `veridata-cloud/aws` | `kms_provider: aws`, `kms_key_id: arn:...` |
| **GCP Cloud KMS** | `veridata-cloud/gcp` | `kms_provider: gcp`, `kms_key_id: projects/.../cryptoKeyVersions/...` |
| **Azure Key Vault** | `veridata-cloud/azure` | `kms_provider: azure`, `azure_vault_url`, `kms_key_id` |

VRP `producer` metadata includes `kms=<provider>/<key_id>` when using cloud signers.

## Warehouse SQL pushdown (AC-D4 production)

| Sink | Pushdown | Notes |
|------|----------|-------|
| `bigquery` | **Yes** (`pushdown_used: true`) | `SELECT TO_HEX(SHA256(...))` in warehouse |
| `iceberg` / `iceberg_s3` | Parquet column projection | Client/object-store read; honest `pushdown_used` |
| `databricks` | Parquet via object store | Same as iceberg_s3 |

## Pipeline integration (not built — common oversell)

These are **not** on `main`. Veridata **detects and proves** faults; it does not operate the pipeline after a FAIL.

| Integration | Status | Notes |
|-------------|--------|-------|
| Inline verification gate before promote | DIY | Call `veridata verify --check` in CI/CD |
| Dead Letter Queue (DLQ) routing | Not done | Route missing/mutated evidence to your DLQ topic |
| Idempotent replay / backfill | Not done | Orchestrator (Airflow, Flink, custom) |
| Auto-remediation agent | Not done | Distinct from AIOps/IaC products |

See [POSITIONING.md](../POSITIONING.md) for accurate enterprise messaging.

## P5 — breadth (post-1.0)

Per `CURSOR-BUILD-SPEC.md` phase P5; not started:

| Feature | ID | Status |
|---------|-----|--------|
| Multi-hop localization | F-B9 | Not done |
| Transform-aware reconciliation | F-B10 | Not done |
| Transparency-log anchoring | F-C8 | Not done |
| Approximate pre-filter | F-B7 | Not done |
| Additional connectors (Pub/Sub→BigQuery, etc.) | F-D5 | Partial — individual connectors exist |
| Continuous / streaming mode | — | Not done |
| Late-arrival **supersede chain** (full F-B8) | F-B8 | Partial — window parse + verify gate only |
| Full commitment recompute without `fp` in proof | — | Partial |

## Suggested implementation order (remaining)

1. GitHub Releases + crates.io (adoption without Docker)
2. Cloud integration tests behind `cloud-integration` feature + Testcontainers
3. P5 features per adopters
