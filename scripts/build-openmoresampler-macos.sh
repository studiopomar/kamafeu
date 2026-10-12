#!/usr/bin/env bash
set -euo pipefail

# Builds OpenMoresampler outside the Kamafeu source tree. The upstream project
# is GPL-3.0 and must remain distributed with its corresponding source.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="${TMPDIR:-/tmp}/kamafeu-openmoresampler-build"
SOURCE_DIR="$WORK_DIR/OpenMoresampler"
BUILD_DIR="$WORK_DIR/build"
OUTPUT_DIR="$ROOT_DIR/resamplers"

command -v cmake >/dev/null || {
  echo "cmake não encontrado. Instale CMake antes de continuar." >&2
  exit 1
}

if ! command -v brew >/dev/null && ! pkg-config --exists libomp 2>/dev/null; then
  echo "OpenMP não encontrado. Instale libomp (por exemplo: brew install libomp)." >&2
  exit 1
fi

mkdir -p "$WORK_DIR" "$OUTPUT_DIR"
if [ ! -d "$SOURCE_DIR/.git" ]; then
  git clone --depth 1 https://github.com/dorayakito/OpenMoresampler "$SOURCE_DIR"
else
  git -C "$SOURCE_DIR" pull --ff-only
fi

# Apple Clang accepts OpenMP through the driver-specific pair below. The
# upstream CMake file uses GCC's bare -fopenmp, so patch only this disposable
# checkout rather than changing or vendoring upstream sources.
if [ "$(uname -s)" = "Darwin" ] && command -v clang >/dev/null; then
  sed -i '' 's/    -fopenmp/    -Xclang -fopenmp/' "$SOURCE_DIR/CMakeLists.txt"
fi

OPENMP_PREFIX=""
if command -v brew >/dev/null; then
  OPENMP_PREFIX="$(brew --prefix libomp 2>/dev/null || true)"
fi

cmake_args=(
  -S "$SOURCE_DIR"
  -B "$BUILD_DIR"
  -DCMAKE_BUILD_TYPE=Release
  -DMORESAMPLER_ARCH=64
)
if [ -n "$OPENMP_PREFIX" ]; then
  cmake_args+=("-DCMAKE_PREFIX_PATH=$OPENMP_PREFIX" "-DOpenMP_ROOT=$OPENMP_PREFIX")
fi

cmake "${cmake_args[@]}"
cmake --build "$BUILD_DIR" --parallel

BUILT="$BUILD_DIR/moresampler"
[ -f "$BUILT" ] || {
  echo "O build terminou sem gerar $BUILT" >&2
  exit 1
}

cp "$BUILT" "$OUTPUT_DIR/openmoresampler"
chmod +x "$OUTPUT_DIR/openmoresampler"
cp "$SOURCE_DIR/LICENSE" "$OUTPUT_DIR/OPENMORESAMPLER-GPL-3.0.txt"

echo "OpenMoresampler instalado em: $OUTPUT_DIR/openmoresampler"
echo "Fonte correspondente preservada em: $SOURCE_DIR"
