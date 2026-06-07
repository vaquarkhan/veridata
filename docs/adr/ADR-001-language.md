# ADR-001: Implementation Language

**Status:** Accepted  
**Date:** 2026-06-07

## Context

veridata requires a deterministic core engine, a single static offline verifier binary, and cross-language interoperability for connectors. The build spec defaults to Rust unless the project owner chooses JVM.

## Decision

Use **Rust** (edition 2021, MSRV aligned with stable toolchain) for:

- `core/` — pure reconciliation engine (no I/O)
- `proof/` — VRP build, sign, verify, chain, store
- `cli/` — operator-facing binary
- `connectors/` — reference Kafka and Iceberg implementations (P2+)

## Consequences

- Workspace layout: Cargo workspace with crates `veridata-core`, `veridata-proof`, `veridata-cli`, etc.
- Tests: `cargo test`, `proptest` for property tests
- CI: `cargo build`, `cargo test`, determinism and architecture gates
- JVM mirror deferred unless explicitly requested

## Alternatives Considered

- **Java 21 + Maven/Gradle:** Strong enterprise adoption; rejected for v1 because Rust better matches determinism, static binary distribution, and dependency-light core goals.
