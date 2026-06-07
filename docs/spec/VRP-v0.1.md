# Verifiable Reconciliation Proof (VRP) v0.1

**Status:** Normative  
**Version:** 0.1  
**Date:** 2026-06-07  
**Tag:** `spec-v0.1`

This document defines the VRP interchange format, canonicalization rules, hash construction, reconciliation semantics, and the offline verification algorithm. An independent implementation MUST produce identical hash values and verdicts given identical inputs and policy.

---

## 1. Scope and guarantees

A VRP is a signed document that attests:

1. Over boundary **B**, the producer collected **source** and **sink** fingerprint multisets.
2. Reconciliation compared those multisets and classified records as matched, missing (drop), duplicated, or mutated.
3. A **policy** evaluated the evidence into verdict `PASS`, `FAIL`, or `UNVERIFIED`.
4. The document is **tamper-evident** via Ed25519 signature over canonical bytes.

**Guarantee (honest):** Verifiable reconciliation with dup/drop/mutation detection and third-party-verifiable proof over a boundary, under the trust assumption that connectors faithfully fingerprint source/sink data per policy.

**Non-guarantee:** Exactly-once delivery, completeness outside the declared boundary, or correctness if connectors mis-fingerprint data.

**Privacy:** VRPs MUST contain only salted hashes. Raw identities and field values MUST NOT appear.

---

## 2. Document structure

Interchange encoding is **canonical JSON** (signing) or **CBOR** (wire/storage). Both MUST map to the same logical fields. Field names below use JSON camelCase where noted; hex fields are lowercase hex strings; binary fields in JSON use base64 (standard, no padding optional — implementations MUST accept unpadded base64).

```json
{
  "vrp_version": "0.1",
  "proof_id": "<hex-sha256-of-signing-payload>",
  "created_at": "<RFC3339-UTC>",
  "producer": "veridata/<semver>",
  "boundary": { "mode": "OFFSET_RANGE", "value": "<base64>" },
  "source_ref": "kafka:orders",
  "sink_ref": "iceberg:warehouse.orders",
  "hash_algorithm": "sha256",
  "canon_version": 1,
  "salt": "<base64-32-bytes>",
  "source_commitment": { "count": 1000, "merkle_root": "<hex>" },
  "sink_commitment":   { "count": 1000, "merkle_root": "<hex>" },
  "reconciliation": {
    "matched": { "count": 1000, "merkle_root": "<hex>" },
    "missing": [
      {
        "id_hash": "<hex>",
        "source_pos": "<base64>",
        "inclusion_proof": ["<hex>"]
      }
    ],
    "duplicated": [
      {
        "id_hash": "<hex>",
        "source_multiplicity": 1,
        "sink_multiplicity": 2
      }
    ],
    "mutated": [
      {
        "id_hash": "<hex>",
        "source_content_hash": "<hex>",
        "sink_content_hash": "<hex>"
      }
    ],
    "verdict": "PASS",
    "unverified_reason": null
  },
  "policy": {
    "identity_rule": "composite:[order_id,line_id]",
    "canon": { "version": 1, "timestamp_precision": "micros", "decimal_scale": 6, "unicode": "NFC", "null_token": "\\u0000", "field_order": "lexicographic", "array_as_set": false },
    "hash_algorithm": "sha256",
    "tolerances": { "max_drops": 0, "duplicates": "FORBID", "max_mutations": 0 },
    "late_arrival_window": "900s"
  },
  "chain": { "prev_proof_hash": null },
  "signature": {
    "alg": "ed25519",
    "public_key": "<base64-32-bytes>",
    "sig": "<base64-64-bytes>"
  }
}
```

### 2.1 Field semantics

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `vrp_version` | string | yes | MUST be `"0.1"` for this spec |
| `proof_id` | hex | yes | SHA-256 of signing payload (see §7) |
| `created_at` | RFC3339 | yes | Non-deterministic; excluded from signing payload hash for determinism tests |
| `producer` | string | yes | Software id, e.g. `veridata/0.1.0` |
| `boundary` | object | yes | Scope of reconciliation (§3) |
| `source_ref` | string | yes | Opaque source locator |
| `sink_ref` | string | yes | Opaque sink locator |
| `hash_algorithm` | enum | yes | `sha256` or `blake3` |
| `canon_version` | u32 | yes | Canonicalization rules version (§4) |
| `salt` | base64 | yes | 32-byte per-proof salt for id/content hashing |
| `source_commitment` | commitment | yes | Source multiset commitment |
| `sink_commitment` | commitment | yes | Sink multiset commitment |
| `reconciliation` | object | yes | Evidence + verdict (§6) |
| `policy` | object | yes | Identity, canon, tolerances (§5) |
| `chain` | object | yes | `prev_proof_hash`: hex or null |
| `signature` | object | yes | Ed25519 signature (§7) |

**Commitment:** `{ "count": u64, "merkle_root": hex32 }` — count is multiset cardinality (sum of multiplicities); root is sorted Merkle over leaf hashes (§8).

**Verdict:** `PASS` | `FAIL` | `UNVERIFIED`. If `UNVERIFIED`, `unverified_reason` MUST be a non-empty string.

---

## 3. Boundary

```text
Boundary { mode: enum, value: bytes }
```

| Mode | Semantics |
|------|-----------|
| `OFFSET_RANGE` | Inclusive start/end offsets per partition; value encodes canonical CBOR map `{partitions: [{id, start, end}]}` |
| `TIME_WINDOW` | `[start, end)` UTC instants; value encodes CBOR `{start: RFC3339, end: RFC3339}` |
| `BATCH_ID` | Opaque batch identifier bytes |

`value` is base64-encoded canonical CBOR. Connectors MUST document encoding for each ref type.

---

## 4. Canonicalization v1 (`canon_version = 1`)

Canonicalization transforms a JSON-like record + field selection into deterministic UTF-8 bytes.

### 4.1 Record model

Records are maps `field_name → value` where values are: null, bool, string, decimal (string encoding), timestamp (RFC3339 string input), array, or nested map. Connectors MUST normalize to this model before hashing.

### 4.2 Rules

1. **Field order:** Fields sorted lexicographically by UTF-8 field name. Only fields in the selected set (content) or identity rule (identity) are included.
2. **Null:** Encoded as the configured `null_token` (default U+0000) wrapped as JSON string content in canon stream: `\x00` → bytes `0x00 0x00` (token length-prefixed — see §4.5).
3. **Empty string:** Distinct from null; encoded as length-prefixed UTF-8 with length 0.
4. **Strings:** Unicode NFC normalization, then UTF-8 bytes.
5. **Decimals:** Parse to rational, scale to `decimal_scale` (default 6) using half-away-from-zero, emit ASCII `[-]?digits.digits` without exponent.
6. **Timestamps:** Parse RFC3339, convert to UTC, truncate to `timestamp_precision` (`micros` = 6 fractional digits). Emit `YYYY-MM-DDTHH:MM:SS.ffffffZ`.
7. **Booleans:** `true` → `0x01`, `false` → `0x00` as single-byte typed values.
8. **Arrays:** Default **list** semantics — order preserved, each element canon-encoded with type tag. If `array_as_set: true`, sort canon bytes of each element lexicographically before concatenation.
9. **Nested maps:** Recurse with same rules; emit map marker, field count, then ordered fields.

### 4.3 Type tags (domain separation inside canon stream)

Each value is prefixed with a type byte:

| Tag | Meaning |
|-----|---------|
| `0xA0` | null |
| `0xA1` | bool |
| `0xA2` | string (u32 BE length + UTF-8) |
| `0xA3` | decimal (u32 BE length + ASCII) |
| `0xA4` | timestamp (u32 BE length + ASCII RFC3339 UTC) |
| `0xA5` | array (u32 BE count + elements) |
| `0xA6` | map (u32 BE field count + fields) |

Field names in maps: `0xF0` + u32 BE length + UTF-8 name + value encoding.

### 4.4 Canon output

`canon(record, selected_fields, canon_version)` → byte string. For unsupported `canon_version`, verifiers MUST return `UNVERIFIED` with reason `unsupported canon_version`.

### 4.5 Identity rule grammar

| Form | Example | Semantics |
|------|---------|-----------|
| `field:<name>` | `field:order_id` | Single-field identity |
| `composite:[f1,f2,...]` | `composite:[order_id,line_id]` | Ordered composite; order matters |

Missing identity field → connector error at fingerprint time; MUST NOT produce a VRP with silent omission.

---

## 5. Policy

```text
Policy {
  identity_rule: string,
  canon: CanonSpec,
  hash_algorithm: enum,
  tolerances: Tolerances,
  late_arrival_window: duration (seconds, ISO-8601 duration or integer seconds string)
}

Tolerances {
  max_drops: u64 (default 0),
  duplicates: FORBID | ALLOW_IF_SINK_IDEMPOTENT,
  max_mutations: u64 (default 0)
}
```

Verdict derivation (§6.4) MUST use the embedded policy, not external config.

---

## 6. Fingerprints and reconciliation

### 6.1 Position

```text
Position { kind: enum, value: bytes }
```

Kinds: `KAFKA_OFFSET`, `PUBSUB_MSGID`, `ICEBERG_ROW`, `CDC_LSN`, `FILE_ROW`.

`value` is opaque, connector-defined, base64 in JSON. Positions are totally ordered within a partition/stream.

### 6.2 Fingerprint

```text
Fingerprint {
  id_hash: [32]byte,
  content_hash: [32]byte,
  fp: [32]byte,
  pos: Position
}
```

### 6.3 Normative hash construction

Let `H` be the configured hash (SHA-256 or BLAKE3). **Domain separation tags are mandatory.**

```text
id_hash      = H(salt || 0x01 || canon(identity_fields, canon_version))
content_hash = H(salt || 0x02 || canon(selected_fields, canon_version))
fp           = H(0x03 || id_hash || content_hash)
merkle_leaf  = H(0x00 || fp)
merkle_node  = H(0x10 || left || right)
```

Never hash raw concatenations without a tag.

### 6.4 Multiset reconciliation

Given source multiset `S` and sink multiset `K` of fingerprints keyed by `id_hash`:

1. **Matched:** For each `id_hash` where `content_hash` agrees and multiplicity `min(s_count, k_count)` > 0, that many copies are matched.
2. **Missing (drop):** Source excess where sink count < source count, or sink absent. Each missing copy lists `id_hash`, `source_pos`, and Merkle inclusion proof against `source_commitment.merkle_root`.
3. **Duplicated:** Where sink count > source count (after mutation handling), record multiplicities.
4. **Mutated:** Same `id_hash`, differing `content_hash` between source and sink. Pair source/sink content hashes; unmatched multiplicities flow to missing/duplicated.

**Verdict:**

```text
if inputs incomparable or insufficient:
  UNVERIFIED + reason
else if missing.count > policy.tolerances.max_drops: FAIL
else if mutated.count > policy.tolerances.max_mutations: FAIL
else if duplicated exists and policy.tolerances.duplicates == FORBID: FAIL
else if duplicated exists and policy.tolerances.duplicates == ALLOW_IF_SINK_IDEMPOTENT: PASS (duplicated still recorded)
else: PASS
```

Never emit `PASS` when verification cannot complete.

### 6.5 Matched commitment

`reconciliation.matched` is its own commitment `{count, merkle_root}` over matched fingerprints' `fp` values (sorted Merkle, §8).

---

## 7. Signing and proof identity

### 7.1 Signing payload

Signature covers **canonical signing bytes** of the document with:

- All fields except `signature`, `created_at`, and `proof_id` (self-referential hash)
- Keys sorted lexicographically (UTF-8)
- No insignificant whitespace
- Numbers as JSON numbers; strings JSON-escaped

Implementations MUST use the same canonical JSON rules as [JCS (RFC 8785)](https://tools.ietf.org/html/rfc8785) for signing.

### 7.2 Signature

- Algorithm: Ed25519
- `public_key`: 32-byte public key, base64
- `sig`: 64-byte signature over signing payload bytes

Verification failure → document invalid (verify returns FAIL).

### 7.3 Proof ID

`proof_id = hex(SHA256(signing_payload_bytes))` — signing payload excludes `created_at`, `signature`, and `proof_id`.

### 7.4 Chain

`chain.prev_proof_hash` links to previous proof's `proof_id` or null. Alteration of a middle proof breaks the chain (detected in P1).

---

## 8. Sorted Merkle tree

Leaves: one per fingerprint, leaf hash = `H(0x00 || fp)`.

1. Sort leaves by lexicographic order of `fp` bytes (tie-break: not applicable — fp is unique per id+content).
2. Build binary tree bottom-up; odd leaf duplicated at each level.
3. Empty set: root = `H(0x00 || zero32)` where `zero32` is 32 zero bytes.

**Inclusion proof:** sibling hashes from leaf to root, hex-encoded.

---

## 9. Offline verify algorithm (pseudocode)

```text
function verify(vrp, trusted_pubkey) -> VerifyResult:

  if vrp.vrp_version != "0.1":
    return FAIL("unsupported version")

  if not schema_valid(vrp):
    return FAIL("schema invalid")

  payload = canonical_signing_bytes(vrp excluding signature, created_at, proof_id)

  if hex(SHA256(payload)) != vrp.proof_id:
    return FAIL("proof_id mismatch")

  if not ed25519_verify(trusted_pubkey, payload, vrp.signature.sig):
    return FAIL("bad signature")

  if vrp.hash_algorithm not in supported_algorithms:
    return UNVERIFIED("unsupported hash_algorithm")

  if vrp.canon_version != 1:
    return UNVERIFIED("unsupported canon_version")

  # Recompute commitments from evidence where possible
  if not verify_merkle_structure(vrp.source_commitment, vrp.reconciliation.missing, "source"):
    return FAIL("source commitment inconsistent")

  if not verify_merkle_structure(vrp.sink_commitment, ...):
    return FAIL("sink commitment inconsistent")

  recomputed_verdict = derive_verdict(vrp.reconciliation, vrp.policy)

  if recomputed_verdict != vrp.reconciliation.verdict:
    return FAIL("verdict mismatch")

  if vrp.reconciliation.verdict == UNVERIFIED and vrp.reconciliation.unverified_reason is empty:
    return FAIL("UNVERIFIED without reason")

  if vrp.reconciliation.verdict == PASS:
    if vrp.source_commitment.count != vrp.sink_commitment.count:
      return FAIL("PASS with unequal counts")
    if vrp.reconciliation.missing.count > 0 and policy forbids:
      return FAIL("PASS with missing")
    # ... additional honesty checks

  return OK(vrp.reconciliation.verdict)


function derive_verdict(reconciliation, policy):
  if reconciliation.unverified_reason is not null:
    return UNVERIFIED
  drops = len(reconciliation.missing)
  muts = len(reconciliation.mutated)
  dups = len(reconciliation.duplicated)
  if drops > policy.tolerances.max_drops: return FAIL
  if muts > policy.tolerances.max_mutations: return FAIL
  if dups > 0 and policy.tolerances.duplicates == FORBID: return FAIL
  return PASS
```

Full implementations MUST also validate inclusion proofs, matched root consistency, and chain linkage when `prev_proof_hash` is set.

---

## 10. CBOR wire format

CBOR encoding mirrors JSON field names as text keys with identical semantics. Signatures always apply to JCS canonical JSON bytes, not CBOR bytes, for v0.1.

---

## 11. Conformance

See `conformance/` directory:

| Vector | Expected verify outcome |
|--------|-------------------------|
| `valid.vrp.json` | PASS |
| `tampered.vrp.json` | FAIL (signature) |
| `drop.vrp.json` | FAIL (verdict + missing evidence) |
| `dup.vrp.json` | FAIL (duplicate evidence) |
| `mutated.vrp.json` | FAIL (mutation evidence) |

Each vector has sibling `.expected.json` consumed by CI.

---

## 12. Security considerations

- Salt MUST be 32 cryptographically random bytes per proof.
- Without salt, id hashes may be vulnerable to dictionary attack — verifiers MUST reject missing/short salt.
- Signing keys MUST be protected; verifiers trust only configured public keys.
- Proofs attest connector faithfulness; malicious connectors can lie. Third-party verification validates internal consistency and signature, not live database state.

---

## 13. References

- SHA-256: FIPS 180-4
- BLAKE3: https://github.com/BLAKE3-team/BLAKE3-specs
- Ed25519: RFC 8032
- JCS: RFC 8785
