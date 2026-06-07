# veridata — Cursor Build Specification (single source of truth for the coding agent)

READ THIS FIRST, AI agent. This file is the complete, self-contained instruction set to build the veridata project. Build strictly phase by phase (P0 -> P5). Do NOT skip ahead. Do NOT add features not listed. After every feature, write and run the tests in its acceptance criteria before moving on. If a requirement is ambiguous, STOP and ask; do not guess.

## 0. What you are building (one paragraph)

veridata is an open framework that produces Verifiable Reconciliation Proofs (VRPs): signed, tamper-evident, independently-verifiable receipts proving that, over a defined boundary, a data sink faithfully reflects a data source — with explicit detection of dropped, duplicated, and silently mutated records. It turns "the dashboard is green" into "the data is provably correct, here is the proof."

## 1. Non-negotiable rules (the agent MUST obey all)

1. **Honesty of claims.** Never describe this as "exactly-once for everything." The guarantee is: verifiable reconciliation + dup/drop/mutation detection + tamper-evident, third-party-verifiable proof over a boundary, under the trust that connectors faithfully fingerprint the data.
2. **Spec before code.** Phase P0 produces the spec + conformance vectors. Implementation (P1+) must conform to it. If code and spec disagree, the spec wins (or you update the spec deliberately with a note).
3. **The core module is pure.** No I/O, no network, no connectors, no clock (except an injected clock). Deterministic: same input -> byte-identical output (except an explicit created_at).
4. **Test every feature.** Each feature has acceptance criteria (Given/When/Then). Write those tests, make them pass, before the next feature. No feature is "done" without green tests.
5. **No silent pass.** Any case the engine cannot fully verify MUST be reported UNVERIFIED with a reason, never PASS.
6. **No raw data in proofs.** Proofs contain only salted hashes. Never serialize raw identities/field values.
7. **Determinism tests are mandatory** and run in CI.
8. **Domain-separate all hashes** (tags below). Never hash raw concatenations without a tag.
9. **Scope discipline.** If a task is not on the path to the current phase's exit criteria, defer it.

## 2. Tech stack decisions (use these defaults unless told otherwise)

- **Language:** Rust for core, proof, cli (default). See ADR-001.
- **Hashing:** SHA-256 default (pluggable). Optional BLAKE3.
- **Signing:** Ed25519.
- **Serialization:** canonical JSON for interchange/spec examples; CBOR for wire/storage. Sign over canonical bytes only.
- **Tests:** Rust: cargo test + proptest. CI: GitHub Actions.
- **First reference path (P2):** Kafka -> Apache Iceberg. See ADR-002.

## 3. Repository layout

See README.md and repo root. Layout created at P0.

## 4. Core data model

See docs/spec/VRP-v0.1.md section 6 for normative hash construction and types.

## 5. PHASE P0 — Specification

**Status: COMPLETE** (2026-06-07, tag `spec-v0.1`)

Deliverables:
- docs/spec/VRP-v0.1.md
- conformance/ vectors + JSON Schema + validate_p0.py
- ADR-001 (Rust), ADR-002 (Kafka->Iceberg)

**P0 EXIT CRITERIA:** met.

## 6. PHASE P1 — Proof engine (deterministic core + verifier) — **NEXT**

Build order (each step = code + tests green before next): model -> hash/canon -> reconciliation -> Merkle -> VRP build -> sign -> verify.

### Features and acceptance tests

- **F-A1** Identity extraction. AC-A1.1 single key; AC-A1.2 composite order matters; AC-A1.3 missing field fails loudly; AC-A1.4 null per policy.
- **F-A2** Content hashing. AC-A2.1 excluded metadata change -> same hash; AC-A2.2 business field change -> different hash; AC-A2.3 only selected_fields matter.
- **F-A3** Canonicalization. AC-A3.1 field-order independence; AC-A3.2 timezone->UTC equality; AC-A3.3 decimal scale equality; AC-A3.4 null != empty string; AC-A3.5 canon_version recorded.
- **F-A5** Salted privacy. AC-A5.1 no raw value recoverable; AC-A5.2 different salts -> non-correlatable id_hash.
- **F-A8** Pluggable hash. AC-A8.1 blake3 selectable; AC-A8.2 unknown algo -> clean verify failure.
- **F-B1** Multiset reconcile. AC-B1.1 identical sets -> PASS; AC-B1.2 equal multiplicity -> matched not dup.
- **F-B2** Drop detection. AC-B2.1 missing listed with id_hash+pos; AC-B2.2 N drops -> missing.count==N.
- **F-B3** Duplicate detection. AC-B3.1 1-vs-2 multiplicities; AC-B3.2 crash-before-ACK extra copy flagged.
- **F-B4** Mutation detection. AC-B4.1 same id diff content -> mutated; AC-B4.2 excluded-field change not flagged.
- **F-B5** Sorted-Merkle + proofs. AC-B5.1 equal matched roots; AC-B5.2 inclusion/absence proof verifies; AC-B5.3 tampered path fails.
- **F-B11** Policy verdict. AC-B11.1 max_drops=0 & 1 drop -> FAIL; AC-B11.2 benign dup allowed -> PASS; AC-B11.3 incomparable -> UNVERIFIED+reason.
- **F-C1** VRP builder. AC-C1.1 all fields per spec; AC-C1.2 byte-identical except created_at.
- **F-C2** Hash chain. AC-C2.1 prev_proof_hash links; AC-C2.2 altered middle proof detected.
- **F-C3** Signing. AC-C3.1 matching pubkey passes; AC-C3.2 wrong key fails.
- **F-C4** Offline verifier. AC-C4.1 valid+pubkey PASS; AC-C4.2 flipped byte FAIL; AC-C4.3 verdict recomputed from evidence.
- **F-C5** Inclusion proofs. AC-C5.1 discrepancy proof validates against root without full data.

### P1 validation

Synthetic fingerprint generator; fault-matrix test; proptest; determinism test; conformance vectors through real verifier; architecture test (core purity).

**P1 EXIT CRITERIA:** all P0-priority ACs green; determinism/property/conformance/architecture gates green.

## 7. PHASE P2 — One real end-to-end path

SPI first, then Kafka -> Iceberg through SPI. Testcontainers E2E with fault injection (drop/dup/mutation). Privacy + reproducibility tests.

**P2 EXIT CRITERIA:** real faults detected + proven + offline-verified; no raw values; boundary reproducible; path via SPI.

## 8. PHASE P3 — Usable + honest benchmarks

CLI, config, CI gate, metrics, keys, proof store, selective hashing, commutative hash. BENCHMARKS.md + demo.

**P3 EXIT CRITERIA:** install->first proof < 30 min; CI gate; published benchmarks; demo recorded.

## 9. PHASE P4 — Publish + standardize (human-led)

Paper skeleton, connector guide, conformance README, InfoQ article draft.

## 10. PHASE P5 — Adoption breadth (post-1.0)

More connectors, continuous mode, multi-hop, alerts, dashboard, transform-aware, anchoring, pre-filter.

## 11. Global validation (every phase)

Test-first per feature; determinism/tamper/honesty/privacy/architecture/conformance gates in CI; 100% coverage target on core + proof/verify.

## 12. Definition of Done for v1

P0 spec + conformance vectors tagged. P1 core + verifier. P2 one real path in CI. P3 CLI + benchmarks + demo. P4 preprint skeleton (+ human milestones).

## 13. Companion documents

When available: 08-BRD.md, 09-DETAILED-DESIGN.md, 03-SPEC-reconciliation-proof.md, 11-ACCEPTANCE-CRITERIA-P1-P5.md, 07-RISKS-and-NON-GOALS.md, 10-FEATURES-AND-PHASES.md.

---

*Full phased build instructions were provided at project inception. P0 artifacts are authoritative for VRP v0.1 format. Proceed to P1.*
