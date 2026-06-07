# veridata

**Verifiable Reconciliation Proofs (VRPs)** — signed, tamper-evident, independently verifiable receipts proving that, over a defined boundary, a data sink faithfully reflects a data source, with explicit detection of dropped, duplicated, and silently mutated records.

> The guarantee is **verifiable reconciliation** with dup/drop/mutation detection and third-party-verifiable proof over a boundary — not "exactly-once for everything."

## Status

| Phase | Scope | Status |
|-------|-------|--------|
| **P0** | VRP v0.1 spec + conformance vectors | In progress |
| P1 | Deterministic core + offline verifier | Planned |
| P2 | Kafka → Iceberg reference path | Planned |
| P3 | CLI, benchmarks, CI gate | Planned |

## Quick links

- [Build specification](CURSOR-BUILD-SPEC.md) — agent/human source of truth for phased delivery
- [VRP v0.1 specification](docs/spec/VRP-v0.1.md) — normative proof format and verify algorithm
- [Conformance vectors](conformance/) — canonical test proofs with expected outcomes
- [ADRs](docs/adr/) — language (Rust), reference path (Kafka→Iceberg)

## What a VRP proves

Given a **boundary** (offset range, time window, or batch id), a VRP commits to:

1. **Source commitment** — count + Merkle root of source fingerprints
2. **Sink commitment** — count + Merkle root of sink fingerprints
3. **Reconciliation evidence** — matched multiset, missing (drops), duplicated, mutated records
4. **Policy verdict** — PASS, FAIL, or UNVERIFIED (never silent pass)
5. **Signature** — Ed25519 over canonical document bytes

Proofs contain **only salted hashes** — never raw field values or identities.

## Verify offline

```bash
# P1+ (not yet available)
veridata verify --pubkey key.pem conformance/valid.vrp.json
```

## License

Apache-2.0 — see [LICENSE](LICENSE).
