"""
Reference offline verifier for VRP v0.1 (§9 subset used by P0 conformance).
"""

from __future__ import annotations

import base64
import hashlib
import json
from dataclasses import dataclass
from typing import Any

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey


@dataclass(frozen=True)
class VerifyResult:
    outcome: str
    reason: str | None = None


def jcs_canonical(obj: Any) -> str:
    def serialize(o: Any) -> str:
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


def signing_payload(doc: dict[str, Any]) -> bytes:
    subset = {
        k: v for k, v in doc.items() if k not in ("signature", "created_at", "proof_id")
    }
    return jcs_canonical(subset).encode("utf-8")


def merkle_node(left: bytes, right: bytes) -> bytes:
    return hashlib.sha256(b"\x10" + left + right).digest()


def verify_merkle_proof_with_index(
    root: bytes, leaf: bytes, proof: list[bytes], index: int
) -> bool:
    current = leaf
    for sibling in proof:
        if index % 2 == 0:
            current = merkle_node(current, sibling)
        else:
            current = merkle_node(sibling, current)
        index //= 2
    return current == root


def verify_merkle_proof(root: bytes, leaf: bytes, proof: list[bytes], leaf_count: int) -> bool:
    if leaf_count == 0:
        return False
    return any(
        verify_merkle_proof_with_index(root, leaf, proof, idx) for idx in range(leaf_count)
    )


def verify_commitment_structure(vrp: dict[str, Any]) -> str | None:
    r = vrp["reconciliation"]
    src = vrp["source_commitment"]["count"]
    snk = vrp["sink_commitment"]["count"]
    matched = r["matched"]["count"]
    missing = len(r["missing"])
    mutated = len(r["mutated"])
    if matched + missing + mutated != src:
        return "source commitment count mismatch"
    dup_excess = sum(
        max(0, d["sink_multiplicity"] - d["source_multiplicity"])
        for d in r["duplicated"]
    )
    if matched + mutated + dup_excess != snk:
        return "sink commitment count mismatch"
    if (
        mutated == 0
        and dup_excess == 0
        and matched == snk
        and r["matched"]["merkle_root"] != vrp["sink_commitment"]["merkle_root"]
    ):
        return "matched/sink merkle root mismatch"
    if (
        r["verdict"] == "PASS"
        and missing == 0
        and mutated == 0
        and not r["duplicated"]
        and (
            vrp["source_commitment"]["merkle_root"] != vrp["sink_commitment"]["merkle_root"]
            or vrp["source_commitment"]["merkle_root"] != r["matched"]["merkle_root"]
        )
    ):
        return "PASS commitment roots diverge"
    return None


def verify_missing_inclusions(vrp: dict[str, Any]) -> str | None:
    missing = vrp["reconciliation"]["missing"]
    if not missing:
        return None
    root = bytes.fromhex(vrp["source_commitment"]["merkle_root"])
    leaf_count = vrp["source_commitment"]["count"]
    for entry in missing:
        leaf = bytes.fromhex(entry["merkle_leaf"])
        proof = [bytes.fromhex(h) for h in entry["inclusion_proof"]]
        if not verify_merkle_proof(root, leaf, proof, leaf_count):
            return "invalid inclusion proof"
    return None


def derive_verdict(reconciliation: dict[str, Any], policy: dict[str, Any]) -> str:
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


def verify_vrp(vrp: dict[str, Any], pubkey_b64: str) -> VerifyResult:
    if vrp.get("vrp_version") != "0.1":
        return VerifyResult("FAIL", "unsupported version")

    payload = signing_payload(vrp)
    proof_id = hashlib.sha256(payload).hexdigest()
    if proof_id != vrp["proof_id"]:
        return VerifyResult("FAIL", "proof_id mismatch")

    try:
        pk = Ed25519PublicKey.from_public_bytes(base64.b64decode(pubkey_b64))
        pk.verify(base64.b64decode(vrp["signature"]["sig"]), payload)
    except (InvalidSignature, Exception):
        return VerifyResult("FAIL", "bad signature")

    recomputed = derive_verdict(vrp["reconciliation"], vrp["policy"])
    if recomputed != vrp["reconciliation"]["verdict"]:
        return VerifyResult("FAIL", "verdict mismatch")

    if vrp["reconciliation"]["verdict"] == "PASS":
        if vrp["source_commitment"]["count"] != vrp["sink_commitment"]["count"]:
            return VerifyResult("FAIL", "PASS with unequal counts")
        if vrp["reconciliation"]["missing"]:
            return VerifyResult("FAIL", "PASS with missing")

    commitment_err = verify_commitment_structure(vrp)
    if commitment_err:
        return VerifyResult("FAIL", commitment_err)

    inclusion_err = verify_missing_inclusions(vrp)
    if inclusion_err:
        return VerifyResult("FAIL", inclusion_err)

    return VerifyResult(vrp["reconciliation"]["verdict"])
