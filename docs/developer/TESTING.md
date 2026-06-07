# Developer testing guide

This guide explains how to run tests, interpret gates, and reach the **100% line-coverage target** on `veridata-core` and `veridata-proof` (especially `proof/verify`).

## Quick start

```bash
# All workspace tests (P1 + P2)
cargo test --workspace

# Property tests (need test-util feature)
cargo test -p veridata-core --features test-util --test proptest_recon

# P0 conformance vectors (Python)
pip install jsonschema cryptography
python conformance/validate_p0.py

# E2E Kafka → Iceberg via SPI
cargo test -p veridata-e2e

# Coverage report (requires cargo-llvm-cov — see below)
./scripts/run-coverage.sh          # Linux/macOS
powershell -File scripts/run-coverage.ps1   # Windows
```

---

## Test philosophy

veridata is **test-first** (see acceptance criteria in the VRP spec and ADRs):

1. Translate each acceptance criterion (AC) into a **Given / When / Then** test before or with the code.
2. No feature is done until its AC tests are green.
3. Mandatory CI gates: determinism, tamper, honesty, privacy, architecture, conformance, coverage.

**Honesty rule:** never emit `PASS` when verification is incomplete — tests must assert `UNVERIFIED` with a reason where applicable.

---

## Test pyramid

```
                    ┌─────────────────────┐
                    │  E2E (veridata-e2e) │  drop/dup/mutation → VRP → verify
                    └──────────┬──────────┘
              ┌────────────────┴────────────────┐
              │  Integration (proof/tests/*)     │  conformance vectors, gates
              └────────────────┬────────────────┘
    ┌─────────────────────────┴─────────────────────────┐
    │  Unit tests (in-module #[test] + core/tests/*)     │  AC per feature
    └─────────────────────────┬─────────────────────────┘
              ┌────────────────┴────────────────┐
              │  P0 conformance (Python JSON)    │  spec-level vectors
              └─────────────────────────────────┘
```

| Layer | Location | Purpose |
|-------|----------|---------|
| Unit | `core/src/**/*.rs`, `proof/src/**/*.rs` | Single-function AC tests |
| Property | `core/tests/proptest_recon.rs` | Random multisets, fault detection |
| Integration | `proof/tests/p1_gates.rs` | VRP build/sign/verify + conformance |
| Architecture | `proof/tests/architecture.rs`, `e2e/tests/architecture.rs` | Layering rules |
| E2E | `e2e/tests/spi_kafka_iceberg.rs` | Full SPI path + fault injection |
| Conformance | `conformance/*.vrp.json` | Spec-level offline verify |

---

## Acceptance-criteria map

Tests are named after AC IDs from the build spec. Use this table to find what to run when changing a module.

### Core — identity & hashing (`veridata-core`)

| AC ID | Feature | Test location |
|-------|---------|---------------|
| AC-A1.1 | Single identity key | `core/src/identity.rs` |
| AC-A1.2 | Composite order matters | `identity.rs`, `canon.rs` |
| AC-A1.3 | Missing field fails | `identity.rs` |
| AC-A1.4 | Null per policy | `identity.rs` |
| AC-A2.1–A2.3 | Content hashing | `core/src/testutil.rs` |
| AC-A3.1–A3.5 | Canonicalization | `core/src/canon.rs` |
| AC-A5.1–A5.2 | Salted privacy | `testutil.rs` |
| AC-A8.1–A8.2 | Pluggable hash | `core/src/hash.rs` |
| AC-B1.1–B1.2 | Multiset reconcile | `core/src/recon.rs` |
| AC-B2.1–B2.2 | Drop detection | `recon.rs` |
| AC-B3.1 | Duplicate detection | `recon.rs` |
| AC-B4.1 | Mutation detection | `recon.rs` |
| AC-B5.1–B5.3 | Sorted Merkle | `hash.rs` |
| AC-B11.1–B11.3 | Policy verdict | `recon.rs` |
| — | Fault matrix | `testutil.rs` |
| — | Property: clean/drop | `core/tests/proptest_recon.rs` |

### Proof — VRP build & verify (`veridata-proof`)

| AC ID | Feature | Test location |
|-------|---------|---------------|
| AC-C1.1–C1.2 | VRP builder + determinism | `proof/tests/p1_gates.rs` |
| AC-C2.1 | Hash chain | `p1_gates.rs` |
| AC-C3.1–C3.2 | Signing | `p1_gates.rs` |
| AC-C4.1–C4.3 | Offline verifier | `p1_gates.rs` |
| AC-C5.1 | Inclusion proofs | `p1_gates.rs` |
| — | Tamper gate | `p1_gates.rs` |
| — | Privacy gate | `p1_gates.rs` |
| — | Honesty gate | `p1_gates.rs` |
| — | Conformance vectors | `p1_gates.rs` |
| — | JCS canonical JSON | `proof/src/format/jcs.rs` |

### Connectors — SPI path (P2)

| AC ID | Feature | Test location |
|-------|---------|---------------|
| AC-A4.1 | Kafka offset reproducible | `connectors/kafka/src/memory.rs` |
| AC-A4.2 | Iceberg snapshot reproducible | `connectors/iceberg/src/connector.rs` |
| AC-A7.1 | Schema drift | `spi/src/schema.rs` |
| AC-D1.1–D1.2 | SPI-only connectors | `kafka/connector.rs`, `iceberg/connector.rs`, `e2e/` |
| AC-D4.1–D4.2 | Pushdown hashing | `iceberg/connector.rs` |
| — | E2E clean/drop/dup/mutation | `e2e/tests/spi_kafka_iceberg.rs` |

---

## Mandatory CI gates

These run in [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml):

| Gate | Command | Pass criteria |
|------|---------|---------------|
| Build | `cargo build --workspace` | Compiles |
| Unit + property | `cargo test --workspace` + proptest | All green |
| P0 conformance | `python conformance/validate_p0.py` | All vectors valid |
| Determinism | `cargo test -p veridata-proof ac_c1_2_deterministic_except_created_at` | Byte-identical except `created_at` |
| Architecture | `cargo test -p veridata-proof --test architecture` | Core has no I/O deps |
| E2E | `cargo test -p veridata-e2e` | Faults detected + verified |
| Coverage | `cargo llvm-cov … --fail-under-lines 100` | 100% lines on core + proof |

Run all gates locally:

```bash
./scripts/run-all-gates.sh        # Linux/macOS
powershell -File scripts/run-all-gates.ps1
```

---

## 100% coverage — setup

Coverage uses **[cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)** (LLVM source-based, works on Linux CI and locally).

### Install

```bash
# One-time install
cargo install cargo-llvm-cov

# Linux: also need llvm-tools (usually via rustup)
rustup component add llvm-tools-preview
```

Windows (GNU toolchain):

```powershell
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

> **Windows note:** `stable-x86_64-pc-windows-gnu` may fail with `can't find crate for profiler_builtins` when instrumenting coverage. Use **WSL2**, **Docker**, or run coverage in **CI** (Ubuntu). MSVC + `llvm-tools-preview` also works if `link.exe` is available.

### Generate report

```bash
# Terminal summary + HTML under target/llvm-cov/html/
./scripts/run-coverage.sh

# Open HTML (Linux)
xdg-open target/llvm-cov/html/index.html
```

```powershell
# Windows
powershell -File scripts/run-coverage.ps1
Start-Process target/llvm-cov/html/index.html
```

### Scope (what must be 100%)

Per build spec §11:

| Crate | Modules | Target |
|-------|---------|--------|
| `veridata-core` | All production code in `core/src/` | **100% lines** |
| `veridata-proof` | All production code in `proof/src/` | **100% lines** |
| Priority | `proof/src/verify/` | Must stay fully covered |

**Excluded from the threshold** (test/support code):

- `core/src/testutil.rs` (behind `test` / `test-util` feature)
- `**/tests/**`, `proof/tests/**`, `e2e/**`
- `connectors/**`, `spi/**` (separate AC coverage; not in 100% gate yet)

### Read the report

1. Open `target/llvm-cov/html/index.html`.
2. Red lines = never executed — add a unit test that hits that branch.
3. Yellow = partial branch coverage — add tests for `else` paths and error cases.

### Fail CI if below 100%

```bash
cargo llvm-cov \
  --package veridata-core \
  --package veridata-proof \
  --lib \
  --features veridata-core/test-util \
  --ignore-filename-regex 'testutil' \
  --fail-under-lines 100 \
  --summary-only
```

---

## Tutorial: add a feature with tests

Example: add a new tolerance rule `max_late_arrivals` (hypothetical).

### Step 1 — Write the failing test

In `core/src/recon.rs` `mod tests`:

```rust
#[test]
fn ac_b11_4_late_arrival_exceeds_tolerance_fails() {
    // Given: policy with max_late_arrivals = 0 and 1 late record
    // When: derive_verdict(...)
    // Then: Verdict::Fail
}
```

Run:

```bash
cargo test -p veridata-core ac_b11_4 -- --nocapture
```

Expect **FAIL** (not implemented yet).

### Step 2 — Implement minimal code

Add logic in `derive_verdict` until the test passes.

### Step 3 — Extend proof layer if VRP fields change

If the VRP JSON shape changes:

1. Update `docs/spec/VRP-v0.1.md` (spec wins).
2. Update `conformance/vrp-0.1.schema.json`.
3. Add/adjust test in `proof/tests/p1_gates.rs`.
4. Regenerate vectors if needed: `python conformance/generate_vectors.py`.

### Step 4 — Check coverage

```bash
./scripts/run-coverage.sh
```

Ensure new lines in `core/` and `proof/` are green.

### Step 5 — Run full gates

```bash
./scripts/run-all-gates.sh
```

---

## Tutorial: add a connector (SPI)

1. Create `connectors/my_sink/Cargo.toml` depending only on `veridata-spi` + `veridata-core`.
2. Implement `SinkConnector` or `SourceConnector`.
3. Add unit tests in `connectors/my_sink/src/...` with `ac_d1_2_uses_spi_only`.
4. Add E2E scenario in `e2e/tests/` if end-to-end proof is required.
5. Verify architecture: `cargo test -p veridata-e2e --test architecture`.

See [connectors/README.md](../connectors/README.md).

---

## Tutorial: debug a failing conformance vector

```bash
# Validate all vectors
python conformance/validate_p0.py

# Single vector through Rust verifier
cargo test -p veridata-proof ac_c4_1_valid_conformance_passes -- --nocapture
```

If Rust verifier fails but Python passes:

- Compare JCS signing bytes (`proof/src/format/jcs.rs` vs `conformance/validate_p0.py`).
- Spec wins — fix Rust or update spec deliberately with a note.

---

## Synthetic test data

Use `veridata-core` test utilities (feature `test-util`):

```rust
use veridata_core::testutil::{
    default_policy, sample_records, fingerprints_from_records,
    inject_drop, inject_dup, inject_mutation, TEST_SALT,
};
```

Enables fault-matrix tests without real Kafka/Iceberg.

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `proptest_recon` not found | `cargo test -p veridata-core --features test-util --test proptest_recon` |
| GNU linker / `rdkafka` on Windows | Default build uses memory Kafka backend; no librdkafka required |
| Coverage tool missing | `cargo install cargo-llvm-cov` |
| `profiler_builtins` on Windows GNU | Run `./scripts/run-coverage.sh` in WSL, or rely on CI Ubuntu job |
| Conformance Python deps | `pip install jsonschema cryptography` |
| Tests pass locally, fail in CI | CI uses `ubuntu-latest`; run gates via Docker or WSL if needed |

---

## Related docs

- [Coverage checklist](COVERAGE-CHECKLIST.md) — per-module 100% targets
- [VRP v0.1 spec](../spec/VRP-v0.1.md)
- [Conformance vectors](../../conformance/README.md)
- [Connector guide](../connectors/README.md)
- [ADRs](../adr/)
