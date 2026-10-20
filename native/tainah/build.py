"""Build the pinned Tainah-rt source for a Kamafeu desktop package."""
import os
import pathlib
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[2]
REVISION = "13385ed68c0d5f4fa84229b76278c70b0f3ad5a7"
URL = "https://github.com/studiopomar/Tainah-rt.git"
SOURCE_OVERRIDE = os.environ.get("TAINAH_RT_SOURCE")
SOURCE = pathlib.Path(SOURCE_OVERRIDE or ROOT / "target" / "tainah-rt-source").resolve()
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
        dirty = subprocess.check_output(
            ["git", "status", "--porcelain"], cwd=SOURCE, text=True
        ).strip()
        if dirty:
            raise SystemExit(
                f"Tainah-rt source has local changes at {current}; expected {REVISION}"
            )
        if SOURCE_OVERRIDE:
            raise SystemExit(
                f"TAINAH_RT_SOURCE revision mismatch: {current}; expected {REVISION}"
            )
        run(["git", "fetch", "origin", REVISION], cwd=SOURCE)
        run(["git", "checkout", "--detach", REVISION], cwd=SOURCE)

run(["git", "submodule", "update", "--init", "--recursive"], cwd=SOURCE)

run([sys.executable, str(SOURCE / "build.py"), "--output", str(PACKAGE)])
