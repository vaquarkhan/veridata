# Positioning — honest enterprise narrative

Veridata targets **systemic adoption in data engineering**, not consumer virality. The right metric is critical pipelines that emit and verify **Verifiable Reconciliation Proofs (VRPs)** — not millions of overnight downloads.

## The problem we solve

The expensive failure mode in data engineering is **silent corruption**: dropped rows, duplicates, and subtle mutations that do not crash the job. Corrupted data then reaches dashboards, models, and regulatory reports.

Veridata produces a **signed, third-party-verifiable receipt** that, over a defined boundary, the sink multiset matches the source under stated policy — with explicit evidence for drops, duplicates, and mutations. Proofs use **salted hashes only**; raw field values never appear in the VRP.

## What Veridata is (v0.1 on `main`)

| Capability | Status |
|------------|--------|
| Deterministic reconcile engine (dup / drop / mutation) | Shipped |
| Sorted Merkle commitments + Ed25519 signatures | Shipped |
| Offline verifier (signature, commitments, inclusion proofs) | Shipped |
| VRP v0.1 spec + P0 conformance vectors | Shipped |
| CLI: init → reconcile → verify → report | Shipped |
| SPI: pluggable `SourceConnector` / `SinkConnector` | Shipped |
| Reference path: **memory Kafka → filesystem Iceberg** | Shipped (demo backends) |
| CI quality gates (Ubuntu): tests, coverage, demo smoke | Shipped |

## What Veridata is not (do not claim these today)

| Claim sometimes made | Reality on `main` |
|----------------------|-------------------|
| “Runs on AWS / GCP / Azure / Databricks production” | **Not yet** — no cloud SDK connectors; see [developer/ROADMAP.md](developer/ROADMAP.md) |
| “10M rows mathematically proved out of the box” | **Mechanism scales in principle**; v0.1 is demo-scale; billion-row needs warehouse SQL pushdown + cloud path |
| “Zero-trust pipeline” (inline gate on every write) | **Philosophy, not product** — you integrate the gate; we ship offline verify + CLI |
| “Automated remediation, DLQ, idempotent replay” | **Not implemented** — we **detect and prove** faults; replay/DLQ is your orchestrator or a future integration |
| “Published on crates.io / PyPI / Docker Hub” | **Not yet** — see P4 roadmap |
| “Real cloud KMS signing” | **File-backed stand-in only** — AWS/GCP/Azure KMS SDKs not wired |

## How architects should describe it

**Accurate one-liner:**

> Veridata generates **cryptographic reconciliation proofs** so teams can prove a data sink faithfully reflects a source over a boundary — and catch silent dup/drop/mutation before bad data poisons downstream analytics.

**Accurate v0.1 scope:**

> The **proof format and engine are production-grade for P0–P3**. **Cloud connectors, distribution, and optional remediation hooks** are the next layer — tracked in the roadmap, not shipped.

## Enterprise adoption path (realistic)

1. **One critical boundary** — e.g. orders topic → lake table — emit a VRP each batch.
2. **Offline verify in CI/CD** — `veridata verify --check` fails the deploy if proof does not pass.
3. **Auditor / third party** — verify with public key only; no access to raw data.
4. **Platform mandate** — every regulated pipeline must attach a VRP; proofs stored in object store (P4).
5. **Standardization** — VRP format in conformance suite; connectors per cloud (P4/P5).

This is how foundational infra spreads in regulated sectors — depth over virality.

## Ecosystem fit (not replacement)

| Layer | Veridata today | Typical owner |
|-------|----------------|---------------|
| Detect + prove mismatch | **Yes** | Veridata |
| Alert / ticket | Integrate (metrics, `--check` exit code) | Datadog, PagerDuty, CI |
| DLQ + replay | **Not built** | Kafka Connect, Airflow, custom jobs |
| IaC / autoscale | **Out of scope** | Terraform, K8s, AIOps tools |

A complete “zero-trust pipeline” story is **Veridata + your orchestration**. We do not auto-rewrite IaC or replay rows today.

## Related ideas (not Veridata v0.1)

These are **separate products or future spikes**, not current scope:

- **AI dataset provenance / watermarking** — same *verifiability* thesis, different surface (training audits).
- **Autonomous cloud IaC remediation** — AIOps; adjacent but not data-fidelity proofs.
- **Domain CV (e.g. garment physics)** — orthogonal venture.

## Further reading

- [developer/PROJECT-STATUS.md](developer/PROJECT-STATUS.md) — verified vs CI-only vs outstanding
- [developer/ROADMAP.md](developer/ROADMAP.md) — P4/P5 implementation backlog
- [spec/VRP-v0.1.md](spec/VRP-v0.1.md) — normative proof format
