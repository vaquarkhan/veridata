<p align="center">
  <img src="docs/assets/veridata-banner.png" alt="Veridata — Verifiable Reconciliation Proofs" width="720">
</p>

# veridata

[![PyPI](https://img.shields.io/pypi/v/veridata-vrp.svg)](https://pypi.org/project/veridata-vrp/)
[![CI](https://github.com/vaquarkhan/veridata/actions/workflows/ci.yml/badge.svg)](https://github.com/vaquarkhan/veridata/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Author:** [Vaquar Khan](https://github.com/vaquarkhan)

**Verifiable Reconciliation Proofs (VRPs)** — signed, tamper-evident, independently verifiable receipts proving that, over a defined boundary, a data sink faithfully reflects a data source, with explicit detection of dropped, duplicated, and silently mutated records.

## Features

### Proof engine
- Normative **VRP v0.1** spec + conformance suite (5 vectors, JSON Schema, Python reference verifier)
- Deterministic reconcile: **drop**, **duplicate**, and **mutation** detection with multiset evidence
- **Sorted Merkle** commitments + **Ed25519** signatures over canonical JSON (JCS)
- Offline verifier: signature, commitments, Merkle inclusion proofs, policy verdict recompute
- **Salted hashes only** in proofs — no raw field values or identities
- Property tests, architecture gates, and **100% line-coverage** CI on `core` + `proof`

### CLI & integration
- `veridata` binary: `init` → `reconcile` → `verify` → `report` → `doctor`
- CI gate: `veridata verify --check` (exit code + `CHECK=OK|FAIL`)
- Prometheus-style metrics export
- Pluggable **SPI**: `SourceConnector` / `SinkConnector`

### Reference path (demo)
- **Memory Kafka** → **filesystem Iceberg** with fault-injected E2E tests
- Demo scripts: `scripts/demo.sh`, `scripts/demo.ps1`

### Cloud (P4) — `cargo build -p veridata-cli --features cloud`
| Platform | Source | Sink | Proof store | Signing |
|----------|--------|------|-------------|---------|
| **AWS** | MSK (IAM) | Iceberg on S3 | S3 | AWS KMS |
| **GCP** | Pub/Sub | BigQuery SQL pushdown | GCS | Cloud KMS |
| **Azure** | Event Hubs | Delta on ADLS | ADLS | Key Vault |
| **Databricks** | — | Delta / Unity Catalog | S3 / ADLS | Cloud KMS |

See [cloud examples](docs/connectors/CLOUD-EXAMPLES.md).

### Python (PyPI: `veridata-vrp`)
- Offline VRP verifier for auditors and CI — no Rust toolchain required
- `pip install veridata-vrp` · `veridata-vrp-verify` CLI · `verify_vrp()` library API

### Supply chain
- Dependabot for Cargo, Python, and GitHub Actions

## What a VRP proves

Given a **boundary** (offset range, time window, or batch id), a VRP commits to:

1. **Source commitment** — count + Merkle root of source fingerprints
2. **Sink commitment** — count + Merkle root of sink fingerprints
3. **Reconciliation evidence** — matched multiset, missing (drops), duplicated, mutated records
4. **Policy verdict** — PASS, FAIL, or UNVERIFIED
5. **Signature** — Ed25519 over canonical document bytes

## Quick start (Python)

```bash
pip install veridata-vrp
veridata-vrp-verify conformance/valid.vrp.json --pubkey conformance/test-key.pub.b64
```

```python
import json
from veridata_vrp import verify_vrp

vrp = json.load(open("proof.vrp.json", encoding="utf-8"))
result = verify_vrp(vrp, pubkey_b64="...")
print(result.outcome)  # PASS | FAIL | UNVERIFIED
```

## Quick start (Rust CLI)

```bash
cargo build -p veridata-cli
cargo run -p veridata-cli -- init
cargo run -p veridata-cli -- reconcile --demo
cargo run -p veridata-cli -- verify
cargo run -p veridata-cli -- report
```

Cloud connectors:

```bash
cargo build -p veridata-cli --features cloud
```

## Documentation

- [VRP v0.1 specification](docs/spec/VRP-v0.1.md)
- [Cloud connector examples](docs/connectors/CLOUD-EXAMPLES.md)
- [Python package](python/README.md)
- [Developer testing guide](docs/developer/TESTING.md)
- [Conformance vectors](conformance/)
- [Roadmap](docs/developer/ROADMAP.md)
- [Contributing](CONTRIBUTING.md)

## License

Apache-2.0 — see [LICENSE](LICENSE).
