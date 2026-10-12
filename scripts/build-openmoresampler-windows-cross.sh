#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="${TMPDIR:-/tmp}/kamafeu-openmoresampler-windows-build"
SOURCE_DIR="$WORK_DIR/OpenMoresampler"
BUILD_DIR="$WORK_DIR/build"
MINGW_PREFIX="$(brew --prefix mingw-w64 2>/dev/null || true)"

[ -n "$MINGW_PREFIX" ] || { echo "mingw-w64 não encontrado; instale com brew install mingw-w64." >&2; exit 1; }
mkdir -p "$WORK_DIR"

if [ ! -d "$SOURCE_DIR/.git" ]; then
  git clone --depth 1 https://github.com/dorayakito/OpenMoresampler "$SOURCE_DIR"
else
  git -C "$SOURCE_DIR" pull --ff-only
fi

# The Homebrew MinGW package does not ship a target OpenMP runtime. Build a
# deterministic single-threaded Windows executable instead; OpenMP pragmas are
# ignored and the small API shim keeps the upstream calls valid.
cp "$ROOT_DIR/scripts/openmoresampler-omp-single.h" "$SOURCE_DIR/omp.h"
sed -i '' 's/    -fopenmp//' "$SOURCE_DIR/CMakeLists.txt"
sed -i '' 's/find_package(OpenMP REQUIRED)/# OpenMP disabled for single-threaded cross-build/' "$SOURCE_DIR/CMakeLists.txt"
sed -i '' 's/target_link_libraries(moresampler PRIVATE OpenMP::OpenMP_C)/# OpenMP runtime omitted/' "$SOURCE_DIR/CMakeLists.txt"

cmake -S "$SOURCE_DIR" -B "$BUILD_DIR" \
  -G "Unix Makefiles" \
  -DCMAKE_TOOLCHAIN_FILE="$ROOT_DIR/scripts/mingw-w64-toolchain.cmake" \
  -DCMAKE_BUILD_TYPE=Release \
  -DMORESAMPLER_ARCH=64
cmake --build "$BUILD_DIR" --parallel

cp "$BUILD_DIR/moresampler.exe" "$ROOT_DIR/resamplers/openmoresampler.exe"
cp "$SOURCE_DIR/LICENSE" "$ROOT_DIR/resamplers/OPENMORESAMPLER-GPL-3.0.txt"
echo "Gerado: $ROOT_DIR/resamplers/openmoresampler.exe"
