# Coverage checklist — `veridata-core` + `veridata-proof`

Use this checklist with the HTML report from `./scripts/run-coverage.sh`. Every **production** line in the modules below must be green (100% line coverage).

> **Excluded:** `core/src/testutil.rs` (test-only helpers behind `test-util` feature).

---

## `veridata-core`

| Module | File | What to cover | Key tests |
|--------|------|---------------|-----------|
| Model | `core/src/model.rs` | Record, Policy, Verdict types | Used by all recon tests |
| Identity | `core/src/identity.rs` | Single/composite keys, null policy | `ac_a1_*` |
| Canon | `core/src/canon.rs` | JCS-like canonical bytes | `ac_a3_*` |
| Hash | `core/src/hash.rs` | SHA-256 tags, Merkle tree | `ac_a8_*`, `ac_b5_*` |
| Recon | `core/src/recon.rs` | Multiset reconcile, verdict | `ac_b1_*`–`ac_b11_*` |
| Error paths | all above | `Err` branches, invalid input | Add `#[test]` per uncovered line |

### Common gaps to watch

- Empty input slices / empty multisets
- `Policy` edge cases (zero tolerance, strict vs lenient)
- Merkle tree with 0, 1, 2, odd number of leaves
- Composite identity with missing optional vs required fields

---

## `veridata-proof`

| Module | File | What to cover | Key tests |
|--------|------|---------------|-----------|
| Format | `proof/src/format/mod.rs`, `types.rs`, `error.rs` | VRP struct, serde, errors | `p1_gates.rs` |
| JCS | `proof/src/format/jcs.rs` | Signing payload bytes | `jcs.rs` unit test |
| Sign | `proof/src/sign/mod.rs` | Ed25519 sign | `ac_c3_*` |
| **Verify** | `proof/src/verify/mod.rs` | **Priority — full offline verify** | `ac_c4_*`, tamper gate |

### Verify module — branch checklist

Ensure tests exist for each path in the verify pseudocode ([VRP-v0.1.md](../spec/VRP-v0.1.md)):

- [x] Valid proof → `PASS` (`ac_c4_1`)
- [x] Bad signature → `FAIL` / tamper (`ac_c4_2`, `tamper_gate_*`)
- [x] Hash chain mismatch → `FAIL` (`tamper_gate_any_flipped_byte_fails`)
- [x] Reconciliation mismatch (drop/dup/mutation) → `FAIL` (`ac_c4_3`)
- [x] Schema / version unsupported → `UNVERIFIED` with reason (`honesty_gate_*`, canon_version gate)
- [x] Missing required VRP fields → `FAIL` or parse error (serde + schema)
- [x] Inclusion proof verification → `ac_c5_2`, `ac_c5_3`; commitment → `ac_c5_3` sink tamper

---

## How to close a gap

1. Run coverage: `./scripts/run-coverage.sh`
2. Open `target/llvm-cov/html/index.html` → navigate to red file
3. Click uncovered line number
4. Add minimal `#[test]` in the same module's `mod tests` or in `proof/tests/p1_gates.rs`
5. Re-run until `--fail-under-lines 100` passes

Example for an error branch:

```rust
#[test]
fn rejects_empty_identity_fields() {
    let err = build_identity_key(&[], &policy).unwrap_err();
    assert!(err.to_string().contains("required"));
}
```

---

## CI enforcement

GitHub Actions runs:

```bash
cargo llvm-cov --package veridata-core --package veridata-proof \
  --lib --features veridata-core/test-util \
  --ignore-filename-regex 'testutil' \
  --fail-under-lines 100 --summary-only
```

PRs that drop coverage below 100% on these crates will fail.
