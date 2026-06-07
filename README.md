# veridata

**Author:** [Vaquar Khan](https://github.com/vaquarkhan)

**Verifiable Reconciliation Proofs (VRPs)** — signed, tamper-evident, independently verifiable receipts proving that, over a defined boundary, a data sink faithfully reflects a data source, with explicit detection of dropped, duplicated, and silently mutated records.

> The guarantee is **verifiable reconciliation** with dup/drop/mutation detection and third-party-verifiable proof over a boundary — not "exactly-once for everything."

## Status

| Phase | Scope | Status |
|-------|-------|--------|
| **P0** | Spec + conformance vectors | Complete |
| **P1** | Deterministic core + offline verifier | Complete |
| **P2** | Kafka → Iceberg via SPI + E2E fault tests | Complete |
| **P3** | CLI, metrics, `--check`, demo | Complete |
| **P4** | Publishing + cloud connectors | Not started — [roadmap](docs/developer/ROADMAP.md) |
| **P5** | Connector breadth + advanced features | Not started — [roadmap](docs/developer/ROADMAP.md) |

## Quick links

- [Project status](docs/developer/PROJECT-STATUS.md) — verified vs CI-only vs outstanding
- [P4/P5 roadmap](docs/developer/ROADMAP.md) — cloud, KMS, SQL pushdown, publishing
- [Developer testing guide](docs/developer/TESTING.md) — run tests, 100% coverage, tutorials
- [Coverage checklist](docs/developer/COVERAGE-CHECKLIST.md) — per-module 100% line targets
- [VRP v0.1 specification](docs/spec/VRP-v0.1.md) — normative proof format and verify algorithm
- [Conformance vectors](conformance/) — canonical test proofs with expected outcomes
- [Benchmarks](BENCHMARKS.md)
- [Contributing](CONTRIBUTING.md)

## What a VRP proves

Given a **boundary** (offset range, time window, or batch id), a VRP commits to:

1. **Source commitment** — count + Merkle root of source fingerprints
2. **Sink commitment** — count + Merkle root of sink fingerprints
3. **Reconciliation evidence** — matched multiset, missing (drops), duplicated, mutated records
4. **Policy verdict** — PASS, FAIL, or UNVERIFIED (never silent pass)
5. **Signature** — Ed25519 over canonical document bytes

Proofs contain **only salted hashes** — never raw field values or identities.

## Quick start (CLI)

```bash
cargo build -p veridata-cli
cargo run -p veridata-cli -- init
cargo run -p veridata-cli -- reconcile --demo
cargo run -p veridata-cli -- verify
cargo run -p veridata-cli -- report
```

Or run the full demo: `powershell -File scripts/demo.ps1`

## Verify offline (library / conformance)

```bash
cargo test -p veridata-proof --test p1_gates
python conformance/validate_p0.py
```

## License

Apache-2.0 — see [LICENSE](LICENSE).
