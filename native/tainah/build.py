"""Build the pinned Tainah-rt source for a Kamafeu desktop package."""
import os
import pathlib
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[2]
REVISION = "8cba8a316947819639d2ad5ecfe679999a98ce4d"
URL = "https://github.com/studiopomar/Tainah-rt.git"
SOURCE = pathlib.Path(
    os.environ.get("TAINAH_RT_SOURCE", ROOT / "target" / "tainah-rt-source")
).resolve()
PACKAGE = ROOT / "target" / "tainah-package"


def run(command, **kwargs):
    subprocess.run(command, check=True, **kwargs)


if not SOURCE.exists():
    SOURCE.parent.mkdir(parents=True, exist_ok=True)
    run(["git", "clone", "--no-checkout", URL, str(SOURCE)])
    run(["git", "checkout", REVISION], cwd=SOURCE)
else:
    current = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=SOURCE, text=True
    ).strip()
    if current != REVISION:
        raise SystemExit(f"Tainah-rt revision mismatch: {current}; expected {REVISION}")

run([sys.executable, str(SOURCE / "build.py"), "--output", str(PACKAGE)])
