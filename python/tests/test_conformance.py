"""Conformance vector tests for the veridata-vrp Python package."""

from __future__ import annotations

import json
from pathlib import Path

import jsonschema
import pytest

from veridata_vrp.schema import load_schema
from veridata_vrp.verify import verify_vrp

REPO_ROOT = Path(__file__).resolve().parents[2]
CONFORMANCE = REPO_ROOT / "conformance"
PUBLIC_KEY = (CONFORMANCE / "test-key.pub.b64").read_text(encoding="utf-8").strip()
SCHEMA = load_schema()

VECTORS = ["valid", "tampered", "drop", "dup", "mutated"]


@pytest.mark.parametrize("name", VECTORS)
def test_conformance_vector(name: str) -> None:
    vrp = json.loads((CONFORMANCE / f"{name}.vrp.json").read_text(encoding="utf-8"))
    expected = json.loads((CONFORMANCE / f"{name}.expected.json").read_text(encoding="utf-8"))

    jsonschema.validate(vrp, SCHEMA)
    result = verify_vrp(vrp, PUBLIC_KEY)

    if name == "tampered":
        assert result.outcome == "FAIL"
    else:
        assert result.outcome == expected["verify_outcome"]
