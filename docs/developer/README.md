# Developer documentation

| Guide | Description |
|-------|-------------|
| [TESTING.md](TESTING.md) | **Start here** — run tests, CI gates, 100% coverage, tutorials |
| [COVERAGE-CHECKLIST.md](COVERAGE-CHECKLIST.md) | Per-module 100% coverage checklist |
| [../connectors/README.md](../connectors/README.md) | Kafka → Iceberg SPI connectors |
| [../spec/VRP-v0.1.md](../spec/VRP-v0.1.md) | Normative VRP format |

## Git hooks (no Cursor attribution)

Before your first commit, install local hooks so commit messages and staged files cannot contain Cursor/agent footers:

```bash
./scripts/install-git-hooks.sh        # Linux/macOS
powershell -File scripts/install-git-hooks.ps1   # Windows
```

`CURSOR-BUILD-SPEC.md` is **gitignored** — keep it locally only; it is not pushed to the remote.

**Author:** Vaquar Khan — see [AUTHORS](../../AUTHORS) and [Cargo.toml](../../Cargo.toml).

## One-liner for new contributors

```bash
cargo test --workspace && python conformance/validate_p0.py && ./scripts/run-coverage.sh
```
