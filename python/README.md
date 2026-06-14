# veridata-vrp (Python)

Offline verifier for **Verifiable Reconciliation Proofs (VRP v0.1)** — schema validation, Ed25519 signature check, commitment arithmetic, and Merkle inclusion proofs.

Published on PyPI as **`veridata-vrp`** (not `veridata` — that name is taken by an unrelated [pandas data-cleaning package](https://pypi.org/project/VeriData/)).

This is the reference Python implementation aligned with [VRP v0.1](../../docs/spec/VRP-v0.1.md). Rust bindings (PyO3) are planned for reconcile/build; this package focuses on **verify** and conformance.

## Install

```bash
pip install veridata-vrp
```

From source (monorepo):

```bash
pip install ./python
```

## Usage

```python
import json
from veridata_vrp import verify_vrp
from veridata_vrp.schema import load_schema

with open("proof.vrp.json", encoding="utf-8") as f:
    vrp = json.load(f)

outcome = verify_vrp(vrp, pubkey_b64="...")
print(outcome.outcome, outcome.reason)
```

CLI:

```bash
veridata-vrp-verify path/to/proof.vrp.json --pubkey path/to/key.pub.b64
```

## Development

```bash
pip install -e "./python[dev]"
pytest python/tests
```

## Publish (maintainers)

```bash
cd python
python -m build
python -m twine upload dist/*
```

Requires PyPI credentials (`TWINE_USERNAME`, `TWINE_PASSWORD` or API token).

## Name on PyPI

| Name | Owner | What it does |
|------|-------|--------------|
| [`veridata`](https://pypi.org/project/VeriData/) | Third party | pandas DataFrame cleaning / validation |
| **`veridata-vrp`** | This repo | Cryptographic VRP offline verifier |

The GitHub product and Rust CLI remain **`veridata`**.
