#!/bin/sh
# Serve clef-flash's /v1/systemone on big, under the GPU lease (issue #109).
# Needs a llama.cpp build with PR #29831 (b11982 used), CUDA 12.8 build plus its cudart tarball.
set -eu
B=${CLEF_LLAMA:-$HOME/.local/llama.cpp/w3-b11982}
D=$B/llama-b11982
S=$(ls -d "$HOME"/.cache/huggingface/hub/models--ggml-org--Clef-Flash-GGUF/snapshots/* | head -1)
export LD_LIBRARY_PATH=$D:$B/cudart-llama-b11982-bin-ubuntu-cuda-12.8-x64
exec gpu-tenant run 3d --who "${WHO:-bevy_grimoire@big}" --wait --timeout 20m -- \
  "$D/llama-server" -m "$S/Clef-Flash-Q4_K_M.gguf" --mmproj "$S/mmproj-Clef-Flash-Q8_0.gguf" \
  -ngl 99 -c 8192 --ubatch-size 4096 --host "${HOST:-127.0.0.1}" --port "${PORT:-8093}" --alias clef-flash
