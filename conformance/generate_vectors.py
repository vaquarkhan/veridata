#!/usr/bin/env python3
"""
P0 throwaway generator for conformance VRP vectors.
Implements normative hash/canon/Merkle/signing enough to produce valid test proofs.
Replaced by veridata-proof in P1; kept for vector regeneration only.
"""
from __future__ import annotations

import base64
import hashlib
import json
import struct
import unicodedata
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat

OUT_DIR = Path(__file__).parent
SCHEMA_PATH = OUT_DIR / "vrp-0.1.schema.json"

# Fixed test key (P0 conformance only — never use in production)
PRIVATE_KEY = Ed25519PrivateKey.from_private_bytes(
    bytes(range(32))  # deterministic test seed
)
PUBLIC_KEY = PRIVATE_KEY.public_key()
PUBLIC_KEY_B64 = base64.b64encode(
    PUBLIC_KEY.public_bytes(Encoding.Raw, PublicFormat.Raw)
).decode("ascii")

SALT = bytes([0xAB] * 32)
SALT_B64 = base64.b64encode(SALT).decode("ascii")

DEFAULT_CANON = {
    "version": 1,
    "timestamp_precision": "micros",
    "decimal_scale": 6,
    "unicode": "NFC",
    "null_token": "\u0000",
    "field_order": "lexicographic",
    "array_as_set": False,
}

DEFAULT_POLICY = {
    "identity_rule": "composite:[order_id,line_id]",
    "canon": DEFAULT_CANON,
    "hash_algorithm": "sha256",
    "tolerances": {
        "max_drops": 0,
        "duplicates": "FORBID",
        "max_mutations": 0,
    },
    "late_arrival_window": "900s",
}

BOUNDARY = {
    "mode": "OFFSET_RANGE",
    "value": base64.b64encode(
        json.dumps(
            {"partitions": [{"id": 0, "start": 0, "end": 99}]},
            separators=(",", ":"),
            sort_keys=True,
        ).encode()
    ).decode("ascii"),
}


def H(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()


def hex32(b: bytes) -> str:
    return b.hex()


def _u32(n: int) -> bytes:
    return struct.pack(">I", n)


def _encode_string(s: str) -> bytes:
    s = unicodedata.normalize("NFC", s)
    raw = s.encode("utf-8")
    return b"\xa2" + _u32(len(raw)) + raw


def _encode_decimal(s: str, scale: int) -> bytes:
    if "." in s:
        whole, frac = s.split(".", 1)
    else:
        whole, frac = s, ""
    frac = (frac + "0" * scale)[:scale]
    canonical = f"{whole}.{frac}" if scale > 0 else whole
    raw = canonical.encode("ascii")
    return b"\xa3" + _u32(len(raw)) + raw


def _encode_value(v: Any, scale: int) -> bytes:
    if v is None:
        return b"\xa0"
    if isinstance(v, bool):
        return b"\xa1" + (b"\x01" if v else b"\x00")
    if isinstance(v, str):
        if v.startswith("ts:"):
            raw = v[3:].encode("ascii")
            return b"\xa4" + _u32(len(raw)) + raw
        if v.startswith("dec:"):
            return _encode_decimal(v[4:], scale)
        return _encode_string(v)
    if isinstance(v, list):
        parts = b"".join(_encode_value(x, scale) for x in v)
        return b"\xa5" + _u32(len(v)) + parts
    raise TypeError(f"unsupported canon value: {v!r}")


def canon(record: dict[str, Any], fields: list[str], scale: int = 6) -> bytes:
    out = b""
    for name in sorted(fields):
        if name not in record:
            raise KeyError(f"missing field {name}")
        out += b"\xf0" + _u32(len(name.encode())) + name.encode("utf-8")
        out += _encode_value(record[name], scale)
    return out


def identity_fields(rule: str) -> list[str]:
    if rule.startswith("composite:["):
        inner = rule[len("composite:[") : -1]
        return [f.strip() for f in inner.split(",")]
    if rule.startswith("field:"):
        return [rule[len("field:") :]]
    raise ValueError(rule)


def fingerprint(
    record: dict[str, Any],
    content_fields: list[str],
    pos_kind: str,
    pos_value: bytes,
    policy: dict,
) -> dict:
    id_f = identity_fields(policy["identity_rule"])
    scale = policy["canon"]["decimal_scale"]
    id_hash = H(SALT + b"\x01" + canon(record, id_f, scale))
    content_hash = H(SALT + b"\x02" + canon(record, content_fields, scale))
    fp = H(b"\x03" + id_hash + content_hash)
    return {
        "id_hash": id_hash,
        "content_hash": content_hash,
        "fp": fp,
        "pos": {"kind": pos_kind, "value": pos_value},
    }


def merkle_leaf(fp: bytes) -> bytes:
    return H(b"\x00" + fp)


def merkle_root(leaves: list[bytes]) -> bytes:
    if not leaves:
        return H(b"\x00" + bytes(32))
    layer = sorted(leaves)
    while len(layer) > 1:
        nxt = []
        for i in range(0, len(layer), 2):
            left = layer[i]
            right = layer[i + 1] if i + 1 < len(layer) else left
            nxt.append(H(b"\x10" + left + right))
        layer = nxt
    return layer[0]


def merkle_proof(sorted_leaves: list[bytes], target: bytes) -> list[str]:
    """Return sibling hashes from leaf to root."""
    if target not in sorted_leaves:
        raise ValueError("leaf not in tree")
    idx = sorted_leaves.index(target)
    layer = list(sorted_leaves)
    proof: list[str] = []
    while len(layer) > 1:
        if len(layer) % 2 == 1:
            layer = layer + [layer[-1]]
        sibling_idx = idx - 1 if idx % 2 == 1 else idx + 1
        proof.append(hex32(layer[sibling_idx]))
        nxt = []
        for i in range(0, len(layer), 2):
            nxt.append(H(b"\x10" + layer[i] + layer[i + 1]))
        idx //= 2
        layer = nxt
    return proof


def pos_b64(kind: str, value: bytes) -> str:
    return base64.b64encode(json.dumps({"kind": kind, "value": value.hex()}).encode()).decode()


def reconcile(source: list[dict], sink: list[dict], policy: dict) -> dict:
    def group(fps: list[dict]) -> dict[bytes, list[dict]]:
        g: dict[bytes, list[dict]] = defaultdict(list)
        for fp in fps:
            g[fp["id_hash"]].append(fp)
        return g

    sg, kg = group(source), group(sink)
    all_ids = set(sg) | set(kg)
    matched_fps: list[bytes] = []
    missing: list[dict] = []
    duplicated: list[dict] = []
    mutated: list[dict] = []

    source_leaves = sorted(merkle_leaf(f["fp"]) for f in source)

    for id_h in sorted(all_ids):
        s_list = list(sg.get(id_h, []))
        k_list = list(kg.get(id_h, []))

        # Pair equal content_hash copies
        i = 0
        while i < len(s_list):
            sj = s_list[i]
            match_idx = next(
                (j for j, kj in enumerate(k_list) if kj["content_hash"] == sj["content_hash"]),
                None,
            )
            if match_idx is not None:
                matched_fps.append(sj["fp"])
                s_list.pop(i)
                k_list.pop(match_idx)
            else:
                i += 1

        # Remaining source with sink present but different content -> mutated
        if s_list and kg.get(id_h):
            pair_count = min(len(s_list), len(k_list))
            for i in range(pair_count):
                mutated.append(
                    {
                        "id_hash": hex32(id_h),
                        "source_content_hash": hex32(s_list[i]["content_hash"]),
                        "sink_content_hash": hex32(k_list[i]["content_hash"]),
                    }
                )
            for sj in s_list[pair_count:]:
                leaf = merkle_leaf(sj["fp"])
                missing.append(
                    {
                        "id_hash": hex32(sj["id_hash"]),
                        "source_pos": pos_b64(sj["pos"]["kind"], sj["pos"]["value"]),
                        "merkle_leaf": hex32(leaf),
                        "inclusion_proof": merkle_proof(source_leaves, leaf),
                    }
                )
        elif s_list:
            for sj in s_list:
                leaf = merkle_leaf(sj["fp"])
                missing.append(
                    {
                        "id_hash": hex32(sj["id_hash"]),
                        "source_pos": pos_b64(sj["pos"]["kind"], sj["pos"]["value"]),
                        "merkle_leaf": hex32(leaf),
                        "inclusion_proof": merkle_proof(source_leaves, leaf),
                    }
                )

        if len(kg.get(id_h, [])) > len(sg.get(id_h, [])):
            duplicated.append(
                {
                    "id_hash": hex32(id_h),
                    "source_multiplicity": len(sg.get(id_h, [])),
                    "sink_multiplicity": len(kg.get(id_h, [])),
                }
            )

    # dedupe mutated entries
    seen_mut: set[tuple] = set()
    mut_unique = []
    for m in mutated:
        key = (m["id_hash"], m["source_content_hash"], m["sink_content_hash"])
        if key not in seen_mut:
            seen_mut.add(key)
            mut_unique.append(m)

    matched_leaves = sorted(merkle_leaf(fp) for fp in matched_fps)
    matched_root = merkle_root(matched_leaves)

    tolerances = policy["tolerances"]
    verdict = "PASS"
    reason = None
    if len(missing) > tolerances["max_drops"]:
        verdict = "FAIL"
    elif len(mut_unique) > tolerances["max_mutations"]:
        verdict = "FAIL"
    elif len(duplicated) > 0 and tolerances["duplicates"] == "FORBID":
        verdict = "FAIL"

    return {
        "matched": {"count": len(matched_fps), "merkle_root": hex32(matched_root)},
        "missing": missing,
        "duplicated": duplicated,
        "mutated": mut_unique,
        "verdict": verdict,
        "unverified_reason": reason,
    }


def jcs_canonical(obj: Any) -> str:
    """Minimal RFC 8785-style canonical JSON."""

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


def signing_payload(doc: dict) -> bytes:
    # proof_id is derived from this payload — must not include itself (§7.1)
    subset = {
        k: v for k, v in doc.items() if k not in ("signature", "created_at", "proof_id")
    }
    return jcs_canonical(subset).encode("utf-8")


def sign_doc(doc: dict) -> dict:
    payload = signing_payload(doc)
    sig = PRIVATE_KEY.sign(payload)
    doc["proof_id"] = hex32(hashlib.sha256(payload).digest())
    doc["signature"] = {
        "alg": "ed25519",
        "public_key": PUBLIC_KEY_B64,
        "sig": base64.b64encode(sig).decode("ascii"),
    }
    return doc


def build_vrp(source: list[dict], sink: list[dict], policy: dict = DEFAULT_POLICY) -> dict:
    source_leaves = sorted(merkle_leaf(f["fp"]) for f in source)
    sink_leaves = sorted(merkle_leaf(f["fp"]) for f in sink)
    recon = reconcile(source, sink, policy)
    doc = {
        "vrp_version": "0.1",
        "proof_id": "0" * 64,
        "created_at": "2026-06-07T00:00:00Z",
        "producer": "veridata/0.1.0-p0",
        "boundary": BOUNDARY,
        "source_ref": "kafka:orders",
        "sink_ref": "iceberg:warehouse.orders",
        "hash_algorithm": "sha256",
        "canon_version": 1,
        "salt": SALT_B64,
        "source_commitment": {
            "count": len(source),
            "merkle_root": hex32(merkle_root(source_leaves)),
        },
        "sink_commitment": {
            "count": len(sink),
            "merkle_root": hex32(merkle_root(sink_leaves)),
        },
        "reconciliation": {
            "matched": recon["matched"],
            "missing": recon["missing"],
            "duplicated": recon["duplicated"],
            "mutated": recon["mutated"],
            "verdict": recon["verdict"],
            "unverified_reason": recon["unverified_reason"],
        },
        "policy": policy,
        "chain": {"prev_proof_hash": None},
        "signature": {"alg": "ed25519", "public_key": PUBLIC_KEY_B64, "sig": ""},
    }
    return sign_doc(doc)


def sample_records(n: int) -> list[dict]:
    rows = []
    for i in range(n):
        rows.append(
            {
                "order_id": str(1000 + i),
                "line_id": str(1),
                "amount": f"dec:{10.5 + i}",
                "status": "shipped",
                "_meta": f"ignore-{i}",
            }
        )
    return rows


CONTENT_FIELDS = ["order_id", "line_id", "amount", "status"]


def fps_from_records(records: list[dict], offset_start: int = 0) -> list[dict]:
    out = []
    for i, r in enumerate(records):
        pos = struct.pack(">Q", offset_start + i)
        out.append(fingerprint(r, CONTENT_FIELDS, "KAFKA_OFFSET", pos, DEFAULT_POLICY))
    return out


def write_vector(name: str, vrp: dict, expected: dict) -> None:
    vrp_path = OUT_DIR / f"{name}.vrp.json"
    exp_path = OUT_DIR / f"{name}.expected.json"
    vrp_path.write_text(json.dumps(vrp, indent=2) + "\n", encoding="utf-8")
    exp_path.write_text(json.dumps(expected, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {vrp_path.name}, {exp_path.name}")


def main() -> None:
    records = sample_records(5)
    src = fps_from_records(records)
    snk = fps_from_records(records)

    valid = build_vrp(src, snk)
    write_vector(
        "valid",
        valid,
        {
            "verify_outcome": "PASS",
            "verdict": "PASS",
            "description": "Identical source and sink multisets",
        },
    )

    tampered = json.loads(json.dumps(valid))
    tampered["signature"]["sig"] = base64.b64encode(b"\xff" * 64).decode("ascii")
    write_vector(
        "tampered",
        tampered,
        {
            "verify_outcome": "FAIL",
            "fail_reason": "bad signature",
            "description": "Valid proof with corrupted signature bytes",
        },
    )

    drop_records = records[:4]
    drop_src = fps_from_records(records)
    drop_snk = fps_from_records(drop_records)
    drop_vrp = build_vrp(drop_src, drop_snk)
    write_vector(
        "drop",
        drop_vrp,
        {
            "verify_outcome": "FAIL",
            "verdict": "FAIL",
            "missing_count": 1,
            "description": "One source record absent from sink",
        },
    )

    dup_records = records + [records[2]]
    dup_src = fps_from_records(records)
    dup_snk = fps_from_records(dup_records)
    dup_vrp = build_vrp(dup_src, dup_snk)
    write_vector(
        "dup",
        dup_vrp,
        {
            "verify_outcome": "FAIL",
            "verdict": "FAIL",
            "duplicated_count": 1,
            "description": "Sink contains duplicate of line_id record",
        },
    )

    mut_records = [dict(r) for r in records]
    mut_records[2] = dict(mut_records[2])
    mut_records[2]["amount"] = "dec:999.99"
    mut_src = fps_from_records(records)
    mut_snk = fps_from_records(mut_records)
    mut_vrp = build_vrp(mut_src, mut_snk)
    write_vector(
        "mutated",
        mut_vrp,
        {
            "verify_outcome": "FAIL",
            "verdict": "FAIL",
            "mutated_count": 1,
            "description": "Same identity, different business field in sink",
        },
    )

    # Write fixed public key for verifiers
    (OUT_DIR / "test-key.pub.b64").write_text(PUBLIC_KEY_B64 + "\n", encoding="utf-8")
    print("done.")


if __name__ == "__main__":
    main()
