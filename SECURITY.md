# Security

**Author:** [Vaquar Khan](https://github.com/vaquarkhan)

## Reporting vulnerabilities

Email or open a **private** security advisory on [github.com/vaquarkhan/veridata](https://github.com/vaquarkhan/veridata/security/advisories/new).

Do not disclose signing keys or production salts in public issues.

## Key handling

- `veridata init` generates Ed25519 keys in `.veridata/keys/`
- **Never commit** `signing.key.b64` or production salts
- The P0 conformance key (`conformance/test-key.pub.b64`) is for tests only
- Rotate keys if a private key is exposed; old proofs remain verifiable with the old public key

## What VRPs guarantee

See [docs/spec/VRP-v0.1.md](docs/spec/VRP-v0.1.md). VRPs prove reconciliation over a declared boundary under the assumption connectors fingerprint faithfully.

## What VRPs do not guarantee

- Exactly-once delivery
- Correctness outside the declared boundary
- Protection against a malicious connector that lies about fingerprints
- Recovery of raw data from proofs (only salted hashes are stored)

## Privacy

Proofs must contain **only salted hashes**. Report any leak of raw identities or field values as a security issue.
