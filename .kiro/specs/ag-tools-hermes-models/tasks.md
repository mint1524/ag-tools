# Tasks — OKAK Hermes model repair

Final acceptance is repeated after explicit api_mode declaration: `20261028_175224_9765f2` (provider default without -m) and `20261028_175224_2ce9aa` (Sonnet alias), recorded in `hermes-evidence.json` under `final_after_api_mode_declaration`.

All four TSK completed; final natural-sync config integrity evidence is in `shared-config-evidence.json`, picker evidence in `picker-evidence.json`, matrix/limits/rollback in `qa-report.md`.


## Progress
| Task | Status | Dependencies | Requirements |
|---|---|---|---|
| TSK-001 | Done | — | NFR-1, NFR-2 |
| TSK-002 | Done | TSK-001 | FR-1, FR-2, NFR-1 |
| TSK-003 | Done | TSK-002 config repair | FR-3, NFR-2 |
| TSK-004 | Done | TSK-003 | all |

- [x] **TSK-001 — Backup and baseline.** Private mode-0600 config/cache backup, sanitized inventory, live catalog, credential file hashes; confirm current provider resolution and failing default baseline. Record exact paths without secrets. Evidence: `config-evidence.json`; old-model CLI session `20261028_173549_dfed13`, exit 1, HTTP 503 after three retries.
- [x] **TSK-002 — Repair shared source/config/catalog.** Reopened after initial work-only repair was reverted by the existing provider-sync timer. Owner explicitly approved root source + synchronized profiles. Back up each eligible home, repair root first with supported CLI, then remove stale OKAK aliases without remapping and add missing exact-ID aliases in the eligible profiles. Refresh only OKAK catalogs. Declare api_mode matching the existing transport so the normal picker matches cache fingerprint; assert the picker exposes the full exact live catalog. Verify complete scope diff/credentials and persistence after a natural timer tick. Initial transient evidence: `config-evidence.json`; final evidence will be `shared-config-evidence.json`.
- [x] **TSK-003 — Real Hermes acceptance.** Fresh normal-rule sessions for OKAK Gemini High and Sonnet 4.6. Prove actual stream/tool-call/result/final fixture marker using recorded session evidence, correct provider/base URL/model, no fallback. PASS: `hermes-evidence.json`, explicit work sessions `20261028_174402_cf5f03` (no model override; provider default selected) and `20261028_174402_0989e6` (Sonnet alias). Each has exactly 2 API calls, 1 read_file, matching tool_call_id and final fixture marker; profile work and endpoint provenance asserted from state.db.
- [x] **TSK-004 — QA and handoff.** Requirements matrix with evidence; update tasks/HQ/project status; commit/push only owned docs/evidence. No production deployment.
