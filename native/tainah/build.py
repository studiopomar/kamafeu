"""Build the pinned Tainah-rt source for a Kamafeu desktop package."""
import os
import pathlib
import shutil
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[2]
REVISION = "141605ee8b6683c14fbf440b5820973b1d809cdd"
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
shutil.copyfile(
    ROOT / "native" / "tainah" / "patches" / "lessConfigure.h",
    engine / "src" / "ConfigUnit" / "lessConfigure.h",
)
shutil.copyfile(
    ROOT / "native" / "tainah" / "patches" / "WorldModule.cpp",
    engine / "src" / "AudioModel" / "WorldModule" / "WorldModule.cpp",
)

# Make WORLD's randn() state thread-local so concurrent multi-core renders
# execute in parallel without global mutex contention while remaining deterministic.
matlab_fn = engine / "lib" / "World" / "src" / "matlabfunctions.cpp"
if matlab_fn.exists():
    matlab_text = matlab_fn.read_text(encoding="utf-8")
    if "static thread_local uint32_t g_randn_" not in matlab_text:
        matlab_text = matlab_text.replace(
            "static uint32_t g_randn_",
            "static thread_local uint32_t g_randn_",
        )
        matlab_fn.write_text(matlab_text, encoding="utf-8")

# The upstream checkout is pinned independently from this application. Keep
# the shipped ABI adapter in sync with the source maintained in this repo.
adapter = ROOT / "native" / "tainah"
adapter_source = SOURCE / "src" / "Tainah"
adapter_source.mkdir(parents=True, exist_ok=True)
for name in (
    "tainah.h",
    "tainah_timing.h",
    "tainah_timing_test.cpp",
    "tainah.cpp",
    "tainah_cli.cpp",
    "tainah_abi_test.cpp",
    "tainah_cli_test.py",
    "CMakeLists.txt",
    "Tainah.yaml",
):
    if (adapter / name).exists():
        shutil.copy2(adapter / name, adapter_source / name)

run([sys.executable, str(SOURCE / "build.py"), "--output", str(PACKAGE)])
