#!/usr/bin/env python3
"""
P0 validation: JSON Schema check + reference verify for conformance vectors.
Uses the `veridata-vrp` PyPI package (install with `pip install -e python`).
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import jsonschema

try:
    from veridata_vrp.schema import load_schema
    from veridata_vrp.verify import verify_vrp
except ModuleNotFoundError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "python" / "src"))
    from veridata_vrp.schema import load_schema
    from veridata_vrp.verify import verify_vrp

CONFORMANCE_DIR = Path(__file__).parent
PUBLIC_KEY_B64 = (CONFORMANCE_DIR / "test-key.pub.b64").read_text(encoding="utf-8").strip()
SCHEMA = load_schema()
VECTORS = ["valid", "tampered", "drop", "dup", "mutated"]


def main() -> int:
    errors = []

    for name in VECTORS:
        vrp_path = CONFORMANCE_DIR / f"{name}.vrp.json"
        exp_path = CONFORMANCE_DIR / f"{name}.expected.json"
        if not vrp_path.exists():
            errors.append(f"missing {vrp_path}")
            continue

        vrp = json.loads(vrp_path.read_text(encoding="utf-8"))
        expected = json.loads(exp_path.read_text(encoding="utf-8"))

        try:
            jsonschema.validate(vrp, SCHEMA)
        except jsonschema.ValidationError as e:
            errors.append(f"{name}: schema invalid: {e.message}")
            continue

        result = verify_vrp(vrp, PUBLIC_KEY_B64)
        exp_outcome = expected["verify_outcome"]

        if name == "tampered":
            if result.outcome != "FAIL":
                errors.append(f"{name}: expected FAIL, got {result.outcome}")
        elif result.outcome != exp_outcome:
            errors.append(
                f"{name}: expected {exp_outcome}, got {result.outcome} ({result.reason})"
            )

        print(
            f"OK {name}: schema valid, verify={result.outcome}"
            + (f" ({result.reason})" if result.reason else "")
        )

    if errors:
        print("\nFAILURES:", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1

    print("\nAll P0 conformance checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
