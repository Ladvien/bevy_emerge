# Serving clef-flash (D1, #109)

**Path that works: llama.cpp `llama-server`, endpoint `POST /v1/systemone`.** Ollama and llama-swap were not tried: `/v1/systemone` is native to llama.cpp (PR #29831, merged 2026-10-03), and a server running a clef model serves only that endpoint (no text generation). Builds on big older than that PR (b8952, `cuda-ece963`, `~/.local/llama.cpp/src`) have no `systemone` string and cannot serve it.

|Item|Value|
|---|---|
|Server|llama.cpp release b11982 (`8a199ad31`), `llama-b11982-bin-ubuntu-cuda-12.8-x64` plus `cudart-llama-b11982-bin-ubuntu-cuda-12.8-x64` (without cudart on `LD_LIBRARY_PATH` the CUDA backend silently does not load and it runs on CPU: 14 s per request)|
|Model|`ggml-org/Clef-Flash-GGUF`: `Clef-Flash-Q4_K_M.gguf` (6.7 GB) and `mmproj-Clef-Flash-Q8_0.gguf` (0.6 GB; needed for images)|
|Command|`tools/decide/serve-clef-flash.sh` (`gpu-tenant run 3d … llama-server -ngl 99 -c 8192 --ubatch-size 4096 --port 8093 --alias clef-flash`)|
|Memory|about 7.4 GB VRAM (11.7 GB total on the card with HomeStill's olmOCR/bge-m3 still resident at 4.3 GB; the card was 19 GB used before the lease), so it fits beside big's other tenants only after the lease drains the 27B coder|
|Latency, fixture (image + 3 questions, 619 input tokens)|first call 0.70 s; five warm calls 0.185–0.195 s (median 0.187 s); load takes about 5 s|

Notes: image tokens are capped at 1024 by `n_ubatch` (server log warns); a clef request reads all questions jointly in one prompt. `/v1/models` reports `output_modalities` containing `decisions`.
