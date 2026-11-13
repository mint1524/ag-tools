# Baseline evidence — 2026-11-13
Read-only SSH nlvmv2 + HTTPS, actual outputs retained in this work session.
- Swarm sidequests-agtools-a1xtnr:1/1; image ghcr.io/mint1524/ag-tools:sha-0066e12; mounted ag_tools_data.
- GET / and /health200; version4.5.1. Auth GET /v1/models200, 79 IDs.
- gemini-3.8-flash-high and claude-sonnet-4-6 direct authenticated POST:429 (~1–2s). Detailed Gemini retry: All accounts exhausted. Last error HTTP403; status PERMISSION_DENIED; ErrorInfo.reason TOS_VIOLATION, domain cloudcode-pa.googleapis.com; Google message: This service has been disabled in this account for violation of Terms of Service. Please submit an appeal to continue using this product.
- Google-supplied appeal URL: https://forms.gle/hGzM9MEUv2azZsrb9 (not submitted).
- One indexed account; disabled/proxy_disabled/validation_blocked=false. Cached quota100% does not override upstream ban. Same reason appears in real app.log.2026-11-13; no RESOURCE_EXHAUSTED markers in examined daily file. Numeric substring log counts were diagnostic only, not status totals.
- Source GitHub latest stable v5.0.3, published2026-11-06; https://github.com/lbjlaq/Antigravity-Manager/releases/tag/v5.0.3. Registry index digest sha256:a10cc6e4580da081b84c86bdd715bd48e7867ad859d8adeecedf266203b70d51, amd64 manifest ef3cdbf48b3de6dcefea838c7ba8120fe4009ad7990d813d77d74b3dc5e71133. Upstream release explicitly improves 403-ban classification and503 attribution; this is not evidence of lifting Google ban.
- Host preflight available8.1G disk,1.5G available RAM; no builds. Existing production-only scope; Hermes files untouched.
