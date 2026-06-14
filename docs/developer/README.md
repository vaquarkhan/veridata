# Developer documentation

| Guide | Description |
|-------|-------------|
| [../POSITIONING.md](../POSITIONING.md) | Enterprise narrative — honest claims vs oversell |
| [PROJECT-STATUS.md](PROJECT-STATUS.md) | Verified vs CI-only vs outstanding work |
| [ROADMAP.md](ROADMAP.md) | P4/P5 — cloud connectors, KMS SDKs, pushdown, publishing |
| [TESTING.md](TESTING.md) | Run tests, CI gates, 100% coverage, Windows linker notes |
| [COVERAGE-CHECKLIST.md](COVERAGE-CHECKLIST.md) | Per-module 100% coverage checklist |
| [../connectors/README.md](../connectors/README.md) | Kafka → Iceberg SPI connectors |
| [../../python/README.md](../../python/README.md) | PyPI package — `pip install veridata-vrp` offline verifier |

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
# No Rust linker? P0 only:
pip install -e ./python
python conformance/validate_p0.py

# Full gates (or use GitHub Actions CI on Ubuntu):
cargo test --workspace && python conformance/validate_p0.py && ./scripts/run-coverage.sh
```
