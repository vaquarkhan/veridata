# Project status — verified vs outstanding

Last updated for `main` (P0–P3 complete; positioning aligned in `docs/POSITIONING.md`).

## Net assessment

The **proof engine and v1 scope (P0–P3)** are in solid shape. Credibility-critical bugs are closed: per-proof random salt, Merkle inclusion verification, commitment structure checks, multiset reconciliation, and CLI/demo path integrity.

The **“runs on real clouds + published”** gap is the major remaining work. That is tracked in [ROADMAP.md](ROADMAP.md) (cloud connectors, cloud KMS SDKs, warehouse SQL pushdown, distribution). Implementation has **not** started beyond design and file-backed stand-ins.

---

## What you can verify without a Rust linker

These run on **Python + JSON only** (no `cargo` required):

| Check | Command | Expected |
|-------|---------|----------|
| P0 conformance (5 vectors) | `python conformance/validate_p0.py` | 5/5 pass — schema + reference verify |
| Vector regeneration | `python conformance/generate_vectors.py` | Writes `conformance/*.vrp.json` |

The Python validator now mirrors Rust verify for: signature, `proof_id`, verdict, **commitment arithmetic**, and **Merkle inclusion proofs**.

---

## What requires a working Rust toolchain (CI if local linker fails)

Full quality gates need `cargo` + linker. **GitHub Actions** (`ubuntu-latest`, `.github/workflows/ci.yml`) is the authoritative runner when local Windows GNU/MSVC toolchains lack a working linker.

| Gate | Command | CI job |
|------|---------|--------|
| Workspace unit/property tests | `cargo test --workspace` | `build-test` |
| P1 architecture | `cargo test -p veridata-proof --test architecture` | `build-test` |
| P2 E2E SPI | `cargo test -p veridata-e2e …` | `build-test` |
| P3 CLI integration | `cargo test -p veridata-cli` | `build-test` |
| 100% core+proof coverage | `cargo llvm-cov … --fail-under-lines 100` | `build-test` |
| CLI demo smoke | `./scripts/demo.sh` | `build-test` |

**Windows without linker:** run `python conformance/validate_p0.py` locally; rely on [CI](https://github.com/vaquarkhan/veridata/actions) for Rust tests, or use WSL2/Docker (see [TESTING.md](TESTING.md)).

---

## Verified by code review (on `main`)

| Area | Location | Notes |
|------|----------|-------|
| Per-proof salt | `core/src/salt.rs`, `cli/src/pipeline.rs` | OS RNG; not `TEST_SALT` |
| Commitment verification | `proof/src/verify/commitments.rs` | Count balance + matched/sink roots |
| Merkle inclusion | `proof/src/verify/mod.rs` | `merkle_leaf` + `verify_merkle_proof` |
| JCS signing | `proof/src/format/jcs.rs` | RFC 8785-style canonical JSON |
| Multiset recon | `core/src/recon.rs` | Zip-pair mutations; proptest |
| Parquet typing | `connectors/iceberg/src/parquet_value.rs` | int64/float/bool/timestamp |
| Column projection pushdown | `connectors/iceberg/src/connector.rs` | Filesystem Parquet only |
| CLI `--check` + metrics | `cli/src/main.rs`, `cli/src/metrics.rs` | AC-E2 / AC-E4 |
| File KMS stand-in | `proof/src/sign/kms.rs` | Not cloud KMS SDKs |
| CBOR wire | `proof/src/format/cbor.rs` | Round-trip; sign still JCS JSON |

---

## Outstanding (not on `main` as implemented features)

See [ROADMAP.md](ROADMAP.md) for detail:

- Cloud connectors (AWS / GCP / Azure / **Databricks**)
- Object-store proof stores (S3, GCS, ADLS)
- Cloud KMS SDKs (vs file-backed stub)
- Warehouse **SQL** pushdown (BigQuery, Spark)
- Publishing (crates.io, GitHub Releases, optional Docker) — **PyPI `veridata-vrp`** ready in `python/`
- P5 breadth (multi-hop, transform-aware, transparency log, …)

---

## Phase table

| Phase | Scope | Status on `main` |
|-------|-------|------------------|
| **P0** | Spec + conformance vectors | Complete |
| **P1** | Core + offline verifier | Complete |
| **P2** | Kafka → Iceberg SPI + E2E | Complete (memory Kafka + filesystem Iceberg) |
| **P3** | CLI, metrics, `--check`, demo | Complete |
| **P4** | Distribution + adoption | **Not done** — see ROADMAP |
| **P5** | Connector breadth + advanced features | **Not done** — see ROADMAP |
