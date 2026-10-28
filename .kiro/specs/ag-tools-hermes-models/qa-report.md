# QA — ag-tools / Hermes models

Verdict: **PASS for the approved configuration repair**, 2026-10-28. No production deployment.

## Requirements / evidence
| Criterion | Result | Evidence |
|---|---|---|
| AC-1.1 — current OKAK provider default, persistent | PASS | `shared-config-evidence.json`: default + 10 profiles, `gemini-3.8-flash-high`; final natural sync completed 17:53:30 CET after final config repair |
| AC-2.1 — exact-ID aliases, no substitution | PASS | 82 aliases per home, exactly the authenticated live catalog; 9 removed per home; retained aliases preserved; `gemini-3.5-flash-lite` added where missing |
| AC-2.2 — normal picker catalog | PASS | `picker-evidence.json`: real shared desktop builder in work, normal `refresh=False`, 1 → 82 models, exact live set |
| AC-3.1 — real Hermes tool loop | PASS | `hermes-evidence.json`, final work sessions `20261028_175224_9765f2` and `20261028_175224_2ce9aa`: correct model/endpoint/profile, 2 API calls and 1 read_file each; matching call ID/result/final marker |
| Default selected without explicit model | PASS | Gemini test uses `--provider okak` without `-m`, not just an explicit working alternative |
| NFR-1 — backup / bounded scope | PASS with runtime-auth observation below | Private config/cache backups 0700/0600; complete parsed config expected-diff assertions on all 11 homes; unrelated provider/global model/alias fields unchanged |
| NFR-2 — no deployment / no secret disclosure | PASS | Only supported config commands and targeted OKAK cache refresh. No service restart, account rotation/removal, production image/volume mutation. Sanitized evidence only |

## Root causes and resolved false starts
1. Old default `gemini-3.6-flash-high` returned HTTP 503 after upstream 429; a real old-model CLI baseline exited 1 after retries (`20261028_173549_dfed13`). Do not infer that all models or the service were down.
2. Work-only config repair was reverted by existing root→profile provider sync (journal 17:38:18). Scope expansion was explicitly approved; root source repaired first. Sync code/timer unchanged.
3. Picker raw discovery hashed absent `api_mode` differently from the warm `openai_chat` cache. Declaring `api_mode=openai_chat` matches the existing transport and restores the full catalog without changing application code.
4. Early successful CLI tests inherited a blank session profile and persisted into the default DB. They were not used as final work acceptance. Final tests explicitly select work and assert its DB/profile_name.

## Credential integrity observation
All credential hashes matched immediately after the shared config repair. Over the longer session, root `auth.json` changed; the existing keepalive journal explicitly records an auth.json update at 17:45:24, and runtime/picker reads may maintain pool metadata. The exact latest root metadata mutation is not exhaustively attributed. No credential restoration, rotation or removal was performed by the repair. At final read-back all `.env` files and every existing named-profile `auth.json` still match initial hashes. Do not claim root auth.json stayed byte-identical throughout the session.

## Acceptance limits
- Real model generation was tested on Gemini High and Sonnet 4.6, not every image/legacy ID advertised by the upstream catalog.
- Live desktop HTTP API probe returned 401 `no_cookie`; authentication was not bypassed. Its shared production model-options builder was executed and verified directly. No visual click-through is claimed.
- Existing conversations retain their selected model; an old 3.6/3.7 model stored in a conversation is not silently rewritten. Select a current OKAK model or start a fresh session.
- Claude 5.5 rollout remains a separate blocked spec; this repair does not change that gate.

## Rollback / files
Private complete snapshots: `/home/mint/.hermes/profiles/work/backups/okak-shared-20261028T164139Z/<profile>/`. Initial work-only snapshot: `.../okak-models-20261028T163527Z/`.
Reverse only owned OKAK config keys through `hermes --profile NAME config set/unset`; restoring the root source propagates via the timer. Do not restore whole stale configs without a concurrency check or restore auth snapshots over live OAuth state.
