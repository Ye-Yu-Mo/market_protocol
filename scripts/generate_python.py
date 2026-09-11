#!/usr/bin/env python3
"""Generate Python bindings from the canonical market protocol schema."""

from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PROTO_ROOT = ROOT / "proto"
PROTO = PROTO_ROOT / "market_protocol/v1/market_protocol.proto"
OUT = ROOT / "python"


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    command = [
        sys.executable,
        "-m",
        "grpc_tools.protoc",
        f"--proto_path={PROTO_ROOT}",
        f"--python_out={OUT}",
        str(PROTO),
    ]
    return subprocess.run(command, cwd=ROOT, check=False).returncode


if __name__ == "__main__":
    raise SystemExit(main())
