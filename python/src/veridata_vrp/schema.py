"""VRP v0.1 JSON Schema (bundled)."""

from __future__ import annotations

import json
from importlib import resources
from typing import Any


def load_schema() -> dict[str, Any]:
    text = resources.files("veridata_vrp.data").joinpath("vrp-0.1.schema.json").read_text(
        encoding="utf-8"
    )
    return json.loads(text)
