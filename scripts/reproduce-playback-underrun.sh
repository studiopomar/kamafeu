#!/usr/bin/env bash
set -euo pipefail

# Reproduces renderer/UI stalls without changing normal playback defaults.
# Override either value when testing a different runway boundary.
: "${KAMAFEU_DEBUG_RENDER_CHUNK_DELAY_MS:=1500}"
: "${KAMAFEU_DEBUG_UI_CHUNK_DELAY_MS:=1000}"

export KAMAFEU_DEBUG_RENDER_CHUNK_DELAY_MS
export KAMAFEU_DEBUG_UI_CHUNK_DELAY_MS

echo "[playback-test] render delay: ${KAMAFEU_DEBUG_RENDER_CHUNK_DELAY_MS}ms"
echo "[playback-test] UI delay: ${KAMAFEU_DEBUG_UI_CHUNK_DELAY_MS}ms"
echo "[playback-test] expected events: WaitingForChunk/Underrun -> PlaybackResumed"

exec cargo run "$@"
