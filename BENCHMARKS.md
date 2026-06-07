# Benchmarks (P3)

Honest performance notes for veridata v0.1. Numbers are indicative — run on your hardware with `./scripts/bench-demo.sh`.

## Methodology

- **Platform:** record OS, CPU, Rust toolchain (`rustc -Vv`)
- **Dataset:** 5 demo records (default `veridata reconcile --demo`); scale with boundary `end` in `recon.yaml`
- **What we measure:** end-to-end reconcile (fingerprint → Merkle → sign → store), offline verify
- **What we do not claim:** sub-millisecond at billion-row scale without pushdown (see ADR-002)

## Reference timings (development machine, debug build)

| Step | Approx. time | Notes |
|------|--------------|-------|
| `veridata init` | < 1 s | Key generation + layout |
| `veridata reconcile --demo` (5 rows) | 1–5 s | Memory Kafka + filesystem Iceberg |
| `veridata verify latest` | < 100 ms | Pure crypto + verdict recompute |
| Full workspace `cargo test` | ~30–60 s | All gates |

Run the demo benchmark:

```bash
./scripts/bench-demo.sh
powershell -File scripts/bench-demo.ps1
```

## Scaling expectations

| Records | Expected dominate cost |
|---------|------------------------|
| < 10k | Canonicalization + hashing |
| 10k–1M | Merkle build + JSON proof size |
| > 1M | Use Iceberg pushdown (`PushdownMode::Pushdown`) — hash in connector, not client |

Future P3+ work: Criterion benches in `core/benches/` and `proof/benches/` for micro-benchmarks of canon, Merkle, and verify.

## Reproducing

```bash
cargo build --release -p veridata-cli
./scripts/demo.sh          # functional demo
./scripts/bench-demo.sh    # prints wall-clock for init/reconcile/verify
```

**Author:** Vaquar Khan
