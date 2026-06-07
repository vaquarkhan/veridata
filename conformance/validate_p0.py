#!/usr/bin/env python3
"""
P0 validation: JSON Schema check + reference verify for conformance vectors.
Reference verifier implements §9 of VRP-v0.1.md (subset sufficient for P0 gates).
"""
from __future__ import annotations

import base64
import hashlib
import json
import sys
from pathlib import Path

import jsonschema
from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

CONFORMANCE_DIR = Path(__file__).parent
SCHEMA = json.loads((CONFORMANCE_DIR / "vrp-0.1.schema.json").read_text(encoding="utf-8"))
PUBLIC_KEY_B64 = (CONFORMANCE_DIR / "test-key.pub.b64").read_text(encoding="utf-8").strip()

VECTORS = ["valid", "tampered", "drop", "dup", "mutated"]


def jcs_canonical(obj) -> str:
    def serialize(o):
        if o is None:
            return "null"
        if isinstance(o, bool):
            return "true" if o else "false"
        if isinstance(o, int):
            return str(o)
        if isinstance(o, str):
            return json.dumps(o, ensure_ascii=False)
        if isinstance(o, list):
            return "[" + ",".join(serialize(x) for x in o) + "]"
        if isinstance(o, dict):
            keys = sorted(o.keys())
            return "{" + ",".join(json.dumps(k) + ":" + serialize(o[k]) for k in keys) + "}"
        raise TypeError(o)

    return serialize(obj)


def signing_payload(doc: dict) -> bytes:
    subset = {
        k: v for k, v in doc.items() if k not in ("signature", "created_at", "proof_id")
    }
    return jcs_canonical(subset).encode("utf-8")


def derive_verdict(reconciliation: dict, policy: dict) -> str:
    if reconciliation.get("unverified_reason"):
        return "UNVERIFIED"
    t = policy["tolerances"]
    if len(reconciliation["missing"]) > t["max_drops"]:
        return "FAIL"
    if len(reconciliation["mutated"]) > t["max_mutations"]:
        return "FAIL"
    if reconciliation["duplicated"] and t["duplicates"] == "FORBID":
        return "FAIL"
    return "PASS"


def verify(vrp: dict, pubkey_b64: str) -> tuple[str, str | None]:
    if vrp.get("vrp_version") != "0.1":
        return "FAIL", "unsupported version"

    payload = signing_payload(vrp)
    proof_id = hashlib.sha256(payload).hexdigest()
    if proof_id != vrp["proof_id"]:
        return "FAIL", "proof_id mismatch"

    try:
        pk = Ed25519PublicKey.from_public_bytes(base64.b64decode(pubkey_b64))
        pk.verify(base64.b64decode(vrp["signature"]["sig"]), payload)
    except (InvalidSignature, Exception):
        return "FAIL", "bad signature"

    recomputed = derive_verdict(vrp["reconciliation"], vrp["policy"])
    if recomputed != vrp["reconciliation"]["verdict"]:
        return "FAIL", "verdict mismatch"

    if vrp["reconciliation"]["verdict"] == "PASS":
        if vrp["source_commitment"]["count"] != vrp["sink_commitment"]["count"]:
            return "FAIL", "PASS with unequal counts"
        if vrp["reconciliation"]["missing"]:
            return "FAIL", "PASS with missing"

    return vrp["reconciliation"]["verdict"], None


def main() -> int:
    pubkey = PUBLIC_KEY_B64
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

        outcome, reason = verify(vrp, pubkey)
        exp_outcome = expected["verify_outcome"]

        if name == "tampered":
            if outcome != "FAIL":
                errors.append(f"{name}: expected FAIL, got {outcome}")
        elif outcome != exp_outcome:
            errors.append(f"{name}: expected {exp_outcome}, got {outcome} ({reason})")

        print(f"OK {name}: schema valid, verify={outcome}" + (f" ({reason})" if reason else ""))

    if errors:
        print("\nFAILURES:", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1

    print("\nAll P0 conformance checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
