# Contributing

**Author:** [Vaquar Khan](https://github.com/vaquarkhan)

Thank you for contributing to veridata.

## Before you start

1. Read [docs/spec/VRP-v0.1.md](docs/spec/VRP-v0.1.md) — the spec wins over code.
2. Read [docs/developer/TESTING.md](docs/developer/TESTING.md) — test-first, mandatory gates.
3. Install git hooks (blocks vendor attribution in commits):

```bash
./scripts/install-git-hooks.sh
powershell -File scripts/install-git-hooks.ps1
```

## Development workflow

```bash
cargo test --workspace
cargo test -p veridata-cli
python conformance/validate_p0.py
./scripts/run-all-gates.sh   # or CI on Ubuntu for coverage
```

## Pull requests

- One logical change per PR
- Include tests for new acceptance criteria (`ac_*` naming)
- No raw field values in proofs
- No Cursor / agent footers in commits or code comments
- Run `./scripts/demo.ps1` for CLI changes

## Code layout

| Crate | Role |
|-------|------|
| `core` | Pure engine (no I/O) |
| `proof` | VRP build / sign / verify |
| `spi` | Connector traits |
| `connectors/*` | Kafka, Iceberg |
| `cli` | Operator commands |

Connectors depend on `spi` + `core` only — never on `proof`.

## Questions

Open a GitHub issue on [vaquarkhan/veridata](https://github.com/vaquarkhan/veridata).
