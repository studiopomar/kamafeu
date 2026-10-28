"""Build the pinned Tainah-rt source for a Kamafeu desktop package."""
import os
import pathlib
import shutil
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[2]
REVISION = "13385ed68c0d5f4fa84229b76278c70b0f3ad5a7"
URL = "https://github.com/studiopomar/Tainah-rt.git"
SOURCE_OVERRIDE = os.environ.get("TAINAH_RT_SOURCE")
SOURCE = pathlib.Path(SOURCE_OVERRIDE or ROOT / "target" / "tainah-rt-source").resolve()
PACKAGE = ROOT / "target" / "tainah-package"
ENGINE_REVISION = "9acdea90da8bb7eb8b2e83f302070364fd838a10"
ENGINE_URL = "https://github.com/dorayakito/lessampler-ng.git"


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

# Prepare the pinned lessampler checkout before its builder runs so our small
# logging fix survives the builder's source copy and remains reproducible.
engine = SOURCE / "build" / "engine-source"
if not engine.exists():
    engine.parent.mkdir(parents=True, exist_ok=True)
    run(["git", "clone", "--no-checkout", ENGINE_URL, str(engine)])
    run(["git", "checkout", ENGINE_REVISION], cwd=engine)
current_engine_revision = subprocess.check_output(
    ["git", "rev-parse", "HEAD"], cwd=engine, text=True
).strip()
if current_engine_revision != ENGINE_REVISION:
    raise SystemExit(
        f"lessampler-ng revision mismatch: {current_engine_revision}; expected {ENGINE_REVISION}"
    )
run(["git", "submodule", "update", "--init", "--recursive"], cwd=engine)
shutil.copy2(ROOT / "native" / "tainah" / "patches" / "LOG.h", engine / "src" / "Utils" / "LOG.h")

# The upstream checkout is pinned independently from this application. Keep
# the shipped ABI adapter in sync with the source maintained in this repo.
adapter = ROOT / "native" / "tainah"
adapter_source = SOURCE / "src" / "Tainah"
adapter_source.mkdir(parents=True, exist_ok=True)
for name in ("tainah.h", "tainah.cpp", "tainah_cli.cpp", "tainah_timing.h"):
    shutil.copy2(adapter / name, adapter_source / name)

# The pinned Tainah-rt builder copies a fixed file list into lessampler-ng.
# Include our timing helper in that copy without maintaining a forked checkout.
builder = SOURCE / "build.py"
builder_text = builder.read_text(encoding="utf-8")
source_list = '("tainah.h", "tainah.cpp", "tainah_cli.cpp", "tainah_abi_test.cpp", "CMakeLists.txt")'
timing_list = '("tainah.h", "tainah_timing.h", "tainah.cpp", "tainah_cli.cpp", "tainah_abi_test.cpp", "CMakeLists.txt")'
if timing_list not in builder_text:
    if source_list not in builder_text:
        raise SystemExit("Tainah-rt builder source list changed; cannot add timing helper")
    builder.write_text(builder_text.replace(source_list, timing_list, 1), encoding="utf-8")

run([sys.executable, str(SOURCE / "build.py"), "--output", str(PACKAGE)])
