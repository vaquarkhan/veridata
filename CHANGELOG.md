# Changelog

All notable changes to veridata. **Author:** Vaquar Khan.

## [0.1.0] — 2026-06-07

### P0 — Specification
- VRP v0.1 normative spec (`docs/spec/VRP-v0.1.md`)
- Conformance vectors + JSON Schema + Python validator
- ADR-001 (Rust), ADR-002 (Kafka → Iceberg)
- Tag: `spec-v0.1`

### P1 — Proof engine
- `veridata-core`: identity, canon, hash, reconcile, Merkle
- `veridata-proof`: VRP build, Ed25519 sign, offline verify
- Determinism, tamper, privacy, architecture gates

### P2 — Reference path
- `veridata-spi`: SourceConnector / SinkConnector
- Memory Kafka + filesystem Iceberg connectors
- E2E fault injection (drop / dup / mutation)

### P3 — CLI
- `veridata` binary: `init`, `reconcile`, `verify`, `report`, `doctor`
- `recon.yaml` configuration
- Filesystem proof store + Ed25519 key files
- Demo and benchmark scripts
- Developer testing guide + 100% coverage gate (CI)

[0.1.0]: https://github.com/vaquarkhan/veridata
