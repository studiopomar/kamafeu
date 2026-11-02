"""Build the pinned Tainah-rt source for a Kamafeu desktop package."""
import os
import pathlib
import shutil
import subprocess
import sys
import textwrap


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


if SOURCE.exists() and not SOURCE_OVERRIDE:
    try:
        source_remote = subprocess.check_output(
            ["git", "remote", "get-url", "origin"], cwd=SOURCE, text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except subprocess.CalledProcessError:
        source_remote = ""
    normalize_url = lambda value: value.removesuffix(".git").rstrip("/").lower()
    if normalize_url(source_remote) != normalize_url(URL):
        # Cargo caches may restore target contents without preserving this
        # nested checkout's .git directory. Never treat the parent repo as the
        # Tainah remote; recreate only this generated target subdirectory.
        generated_source = (ROOT / "target" / "tainah-rt-source").resolve()
        if SOURCE != generated_source:
            raise SystemExit(f"refusing to replace unexpected Tainah source path: {SOURCE}")
        shutil.rmtree(SOURCE)

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
        try:
            run(["git", "fetch", "origin", REVISION], cwd=SOURCE)
        except subprocess.CalledProcessError:
            # Some cached shallow clones reject fetching a commit by object ID
            # even when it is the public default-branch HEAD. Fetch the branch,
            # then enforce the same exact pin before checking it out.
            run(["git", "fetch", "origin", "main"], cwd=SOURCE)
            fetched = subprocess.check_output(
                ["git", "rev-parse", "FETCH_HEAD"], cwd=SOURCE, text=True
            ).strip()
            if fetched != REVISION:
                raise SystemExit(
                    f"Tainah-rt main moved to {fetched}; expected pinned revision {REVISION}"
                )
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

if os.environ.get("TAINAH_SKIP_RUNTIME_TESTS") == "1":
    # Cross architecture Windows runners can compile ARM64, but cannot execute
    # the resulting ABI tests or CLI smoke binary on the x64 host.
    upstream_builder = SOURCE / "build.py"
    builder_text = upstream_builder.read_text(encoding="utf-8")
    ctest_block = '''    run([
        "ctest", "--test-dir", str(build), "-C", "Release",
        "-R", "tainah_(abi|timing)", "--output-on-failure",
    ])'''
    if ctest_block not in builder_text:
        raise SystemExit("não foi possível adaptar os testes runtime do Tainah-rt para cross-build")
    builder_text = builder_text.replace(
        ctest_block,
        '    if os.environ.get("TAINAH_SKIP_RUNTIME_TESTS") != "1":\n'
        + textwrap.indent(ctest_block, "    "),
        1,
    )
    smoke_start = '    test_dir = work / "cli-smoke-test"'
    smoke_end = "    print(package)"
    start = builder_text.find(smoke_start)
    end = builder_text.find(smoke_end, start)
    if start < 0 or end < 0:
        raise SystemExit("não foi possível adaptar o smoke test CLI do Tainah-rt para cross-build")
    smoke_block = builder_text[start:end]
    builder_text = (
        builder_text[:start]
        + '    if os.environ.get("TAINAH_SKIP_RUNTIME_TESTS") == "1":\n'
        + '        print("testes de execução ignorados: alvo Windows ARM64 cross-compilado")\n'
        + "    else:\n"
        + textwrap.indent(smoke_block, "    ")
        + builder_text[end:]
    )
    upstream_builder.write_text(builder_text, encoding="utf-8")

run([sys.executable, str(SOURCE / "build.py"), "--output", str(PACKAGE)])
