"""CLI entry point for offline VRP verification."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import jsonschema

from veridata_vrp.schema import load_schema
from veridata_vrp.verify import verify_vrp


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Verify a VRP v0.1 proof offline")
    parser.add_argument("vrp", type=Path, help="Path to .vrp.json proof file")
    parser.add_argument(
        "--pubkey",
        type=Path,
        required=True,
        help="Path to Ed25519 public key (base64, one line)",
    )
    parser.add_argument(
        "--skip-schema",
        action="store_true",
        help="Skip JSON Schema structural validation",
    )
    args = parser.parse_args(argv)

    vrp = json.loads(args.vrp.read_text(encoding="utf-8"))
    pubkey_b64 = args.pubkey.read_text(encoding="utf-8").strip()

    if not args.skip_schema:
        try:
            jsonschema.validate(vrp, load_schema())
        except jsonschema.ValidationError as exc:
            print(f"schema invalid: {exc.message}", file=sys.stderr)
            return 1

    result = verify_vrp(vrp, pubkey_b64)
    if result.reason:
        print(f"{result.outcome}: {result.reason}")
    else:
        print(result.outcome)

    return 0 if result.outcome in ("PASS", "FAIL", "UNVERIFIED") else 1


if __name__ == "__main__":
    sys.exit(main())
