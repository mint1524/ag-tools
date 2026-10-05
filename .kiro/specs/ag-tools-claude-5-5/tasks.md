---
feature: ag-tools-claude-5-5
phase: tasks
status: approved
approved_by: owner
approved_at: 2026-10-05
---

# Tasks: AG Tools → Claude 5.5

## Status legend

- `[ ]` Pending
- `[>]` In progress
- `[x]` Done with evidence
- `[~]` Partially done / blocked by an explicit gate
- `[-]` Not applicable by recorded decision

## Progress

| Task | Phase | Status | Depends on | Gate / evidence |
|---|---|---:|---|---|
| TSK-001 | Spec | Done | — | Requirements, design and tasks approved; mechanical consistency check |
| TSK-002 | Supply-chain proof | Done | TSK-001 | `evidence/tsk-002-supply-chain.md` — tag/digest/catalog/fix lineage PASS |
| TSK-003 | Baseline + backup | Done | TSK-001 | `evidence/tsk-003-baseline-backup.md` — restorable backup PASS |
| TSK-004 | Shadow isolation | Done | TSK-002, TSK-003 | `evidence/tsk-004-shadow-isolation.md` — isolation/health PASS |
| TSK-005 | Data compatibility | Done | TSK-004 | `evidence/tsk-005-data-compatibility.md` — two-start preservation PASS |
| TSK-006 | Catalog + Gemini | Blocked | TSK-005 | `evidence/tsk-006-007-capability-blocker.md` — Gemini PASS, Claude catalog 0/6 |
| TSK-007 | Claude capability | Blocked | TSK-006 | Upstream quota refresh omits 5.5; direct 5.5 attempts return upstream 404 |
| TSK-008 | Hermes inventory | Done | TSK-001 | `evidence/tsk-008-hermes-inventory.md` — three qualified profiles; writes deferred |
| TSK-009 | Independent review | In progress | TSK-002–TSK-008 | Review the blocker/evidence; cannot authorize deployment while TSK-006/007 are red |
| TSK-010 | Owner deploy gate | Blocked | TSK-009 | Closed by missing 5.5 catalog/capability; no Deploy guide issued |
| TSK-011 | Production read-back | Blocked | TSK-010 + owner Deploy | Runtime, accounts, models and generation verified live |
| TSK-012 | Hermes alias activation | Blocked | TSK-011 | Six aliases/cache verified in every qualified profile |
| TSK-013 | Rollback readiness/action | Pending | TSK-003; conditional on TSK-011 | Previous image + backup restoration path; execute only on failure |
| TSK-014 | QA closeout | Blocked | TSK-011–TSK-013 | QA report, tasks matrix, HQ project/checkpoint current |

**Progress:** 6 / 14 tasks done. TSK-006/007 block the production gate; TSK-011, TSK-012 and TSK-014 also require the owner-only production deployment.

---

## Epic 1 — Approved specification and immutable input

### TSK-001. Finalize and mechanically validate the feature spec

- [x] Requirements, design and task breakdown are present under `.kiro/specs/ag-tools-claude-5-5/`.
- [x] Owner decisions are recorded: clean upstream v4.9.4, all six IDs, all existing OKAK profiles, manual owner Deploy.
- [x] Check FR/NFR/AC and TSK identifiers for duplicates/gaps.
- [x] Check that every requirement has at least one task in the coverage matrix.
- [x] Record the three approved phases in the HQ checkpoint.

**Dependencies:** none.

**Definition of Done / evidence:** all three spec files read back; consistency script exits 0; spec and HQ commits contain only intended files.

**Requirements:** all (planning/control task).

### TSK-002. Prove immutable candidate and upstream capability

- [x] Read back the v4.9.4 tag/commit from upstream.
- [x] Read back the multi-platform registry digest and ensure it equals `sha256:eef6a4d326d420429b3a31b9f3cc1a990fcc462704afd9aa2d58d74224cf1e47`.
- [x] Pull by immutable digest on `nlvmv2`; inspect the local repo digest without running against live data.
- [x] Parse `official_models.json` at the exact v4.9.4 ref and assert the approved six-ID set.
- [x] Confirm the signature-fix lineage and v4.6.3+ account JSON self-heal lineage from source/commit evidence.
- [x] Confirm account/config structures do not deny unknown fork fields.

**Dependencies:** TSK-001.

**Definition of Done / evidence:** one secret-free machine-readable evidence file or task note contains tag SHA, registry digest, six-ID set and relevant upstream commits/source paths; every assertion is reproduced against immutable refs.

**Requirements:** FR-1; AC-1.1; NFR-3.

---

## Epic 2 — Production-safe backup and shadow runtime

### TSK-003. Capture live baseline and create verified pre-migration backup

- [x] Re-read Swarm service image, digest, replica state and restart status.
- [x] Re-check production health/version and bounded Gemini completion.
- [x] Determine the exact live volume/mount from `docker service inspect`/`docker inspect`; do not assume the name.
- [x] Create a timestamped root-only backup while the service remains running, using a consistency-safe copy strategy for the actual volume.
- [x] Test archive/extraction, parse `accounts.json` and all account JSON files, and assert secret-safe invariants: count 3, Google-only, disabled 0, proxy-disabled 0.
- [x] Record the backup path, size and checksum in evidence without account IDs/emails/tokens.

**Dependencies:** TSK-001.

**Definition of Done / evidence:** live service still `1/1`; backup exists and verifies; JSON parse/invariants pass; exact previous image is recorded; no production restart/deploy occurred.

**Requirements:** FR-2; AC-2.1; FR-3; AC-3.1, AC-3.2; FR-6; AC-6.1; FR-9; AC-9.1, AC-9.2; NFR-1, NFR-2, NFR-3.

### TSK-004. Start and inspect isolated v4.9.4 shadow candidate

- [x] Restore the verified backup into a new dedicated shadow volume.
- [x] Verify a free loopback port and start the pinned digest with no restart policy, no Traefik labels and no Dokploy ownership.
- [x] Inspect mounts and published ports; fail if the live volume or a non-loopback bind is present.
- [x] Wait for real readiness and assert `/health` HTTP 200 with version `4.9.4`.
- [x] Preserve bounded candidate logs with secret redaction/absence checks.

**Dependencies:** TSK-002, TSK-003.

**Definition of Done / evidence:** container inspection proves dedicated shadow volume, loopback-only port, no Traefik labels, pinned digest and healthy v4.9.4 response.

**Requirements:** FR-1; AC-1.2; FR-2; AC-2.2, AC-2.3; NFR-1, NFR-2, NFR-3.

### TSK-005. Verify data compatibility across first start and restart

- [x] Parse the shadow account index/files after first startup and assert the preservation invariant.
- [x] Stop the disposable shadow cleanly, start it again on the same copy, and repeat health/account checks.
- [x] Compare first-start and second-start secret-safe counts/states.
- [x] Confirm no malformed/quarantined account was silently lost.

**Dependencies:** TSK-004.

**Definition of Done / evidence:** both startups show health 200/version 4.9.4 and exactly three enabled Google accounts with zero proxy-disabled; parse errors/quarantine count is zero.

**Requirements:** FR-3; AC-3.1, AC-3.2, AC-3.3; NFR-1, NFR-2, NFR-3.

---

## Epic 3 — Model and request-path verification

### TSK-006. Verify the live candidate catalog and Gemini regression

- [x] Call shadow `/v1/models` with an existing credential without printing it.
- [~] Programmatically compare `data[].id` to the approved six-ID set; result is 0/6, so the acceptance assertion is red.
- [x] Assert `gemini-3.8-flash-low` remains in the catalog.
- [x] Send one bounded Gemini `Reply exactly: OK` completion and validate HTTP/status/content.

**Dependencies:** TSK-005.

**Definition of Done / evidence:** machine-parsed output reports six-of-six Claude IDs exactly once, Gemini model present and Gemini completion HTTP 200/`OK`.

**Requirements:** FR-4; AC-4.1, AC-4.2; FR-6; AC-6.1, AC-6.2; NFR-1, NFR-3, NFR-4.

### TSK-007. Verify all Claude 5.5 variants and the multi-turn fix

- [~] Send one bounded `claude-sonnet-5-5-high` probe: client HTTP 503; redacted logs prove upstream HTTP 404 across all three accounts. Do not burn quota on the other five while capability is absent.
- [x] Record only model/status/error classification; no account identifier, hidden reasoning or credential was emitted.
- [ ] Run a two-message conversation proof for `claude-sonnet-5-5-high` — blocked until single-turn capability appears.
- [ ] Run a two-message conversation proof for `claude-opus-5-5-high` — blocked until single-turn capability appears.
- [x] Search bounded responses/logs for `thinking.signature` failures; zero, but generation did not reach a successful response.
- [x] Classify the upstream 404/account-catalog absence as an entitlement/rollout blocker and stop the production path.

**Dependencies:** TSK-006.

**Definition of Done / evidence:** six HTTP 200 single-turn results, two HTTP 200 second-turn results, non-empty assistant content and zero signature errors. Any entitlement failure leaves this task not done and blocks TSK-010.

**Requirements:** FR-5; AC-5.1, AC-5.2, AC-5.3, AC-5.4; NFR-1, NFR-3, NFR-4.

---

## Epic 4 — Hermes preparation and review

### TSK-008. Inventory Hermes profiles and prepare alias activation

- [x] Discover default and named Hermes homes without printing secrets.
- [x] Parse each profile and classify whether an existing `providers.okak` targets `https://ai.okak.club/v1`.
- [x] Capture each qualified profile’s current default provider/model and existing relevant aliases.
- [x] Generate exact `hermes config set` commands for six aliases per qualifying profile using that profile’s own context.
- [x] Verify the plan introduces no provider/credential into unqualified profiles and does not change defaults.
- [x] Do **not** activate aliases/cache before production serves the IDs.

**Dependencies:** TSK-001.

**Definition of Done / evidence:** secret-free inventory lists qualified profile names, before-state defaults and exact planned mappings; negative set is recorded by count/name; current configs are unchanged.

**Requirements:** FR-7; AC-7.1, AC-7.3, AC-7.4; NFR-1, NFR-5.

### TSK-009. Run an independent fail-closed review

- [ ] Commit/push the spec and secret-free evidence before review.
- [ ] Dispatch an approved independent reviewer in read-only mode against the spec, evidence and current runtime facts.
- [ ] Explicitly prohibit production deployment, secret/config-value reads, shared-tree mutation and access to `.env`, account JSON values or key files.
- [ ] Reproduce every finding against current state.
- [ ] Fix all applicable findings, rerun affected checks and obtain a clean final verdict; 401/429/tool failure is not PASS.

**Dependencies:** TSK-002 through TSK-008.

**Definition of Done / evidence:** final PASS with no open suggestions, or an explicitly recorded external review blocker plus exact parent-side reproduction (the latter does not satisfy the normal independent-review gate unless owner accepts it).

**Requirements:** FR-1–FR-8; NFR-1–NFR-5.

---

## Epic 5 — Owner deployment gate

### TSK-010. Hand over the verified candidate for manual Dokploy deployment

- [ ] Re-read the production application/service and confirm it still uses the old image before handoff.
- [ ] Provide the exact pinned image reference and exact Dokploy application/navigation steps.
- [ ] State which fields must remain unchanged: volume, domain, port and environment.
- [ ] Provide health/account/model rollback triggers and the previous immutable image.
- [ ] Stop and wait for explicit owner confirmation that Deploy completed.

**Dependencies:** TSK-009.

**Definition of Done / evidence:** owner receives an exact click guide and candidate evidence; audit confirms the agent made no Dokploy deploy/redeploy/update-image call.

**Requirements:** FR-8; AC-8.1; FR-9; AC-9.1, AC-9.2; NFR-2, NFR-3.

### TSK-011. Verify production after the owner deploys

- [ ] Read back actual Swarm image digest, replicas, running task and restart count.
- [ ] Verify `/health` HTTP 200/version 4.9.4.
- [ ] Verify the three-account preservation invariant without exposing identifiers.
- [ ] Verify live `/v1/models` includes all six IDs and required Gemini model.
- [ ] Run bounded Gemini, six Claude single-turn and two Claude second-turn production smokes.
- [ ] Check bounded production logs for migration/signature/restart errors.

**Dependencies:** TSK-010 and explicit owner confirmation that manual Deploy completed.

**Definition of Done / evidence:** all live read-backs and smokes pass against `https://ai.okak.club/v1`; no success is declared from Dokploy status alone.

**Requirements:** FR-1, FR-3–FR-6, FR-8; AC-1.2, AC-3.1–AC-3.3, AC-4.1–AC-4.2, AC-5.1–AC-5.4, AC-6.1–AC-6.2, AC-8.2; NFR-1–NFR-4.

---

## Epic 6 — Hermes activation, rollback and QA

### TSK-012. Activate and verify six OKAK aliases in qualified Hermes profiles

- [ ] Re-read the qualifying profile set from TSK-008.
- [ ] For each qualifying profile, use `hermes config set` (with correct profile context) to add the six exact `{provider: okak, model: id}` mappings.
- [ ] Refresh/remove only the relevant stale provider-model cache through the supported Hermes mechanism.
- [ ] Read back all six mappings, unchanged defaults and live six-ID discovery.
- [ ] Re-scan unqualified profiles and prove no OKAK provider/alias was introduced.

**Dependencies:** TSK-011.

**Definition of Done / evidence:** every and only qualified profile has six correct aliases, unchanged defaults and a live catalog containing the targets; credential boundaries remain profile-local.

**Requirements:** FR-7; AC-7.1, AC-7.2, AC-7.3, AC-7.4, AC-7.5; NFR-1, NFR-5.

### TSK-013. Prove rollback readiness and execute rollback only if triggered

- [ ] Before the owner gate, validate that the previous image reference and backup artifact are readable.
- [ ] Document exact owner rollback clicks and the data-restore condition.
- [ ] If TSK-011 fails, instruct the owner to restore the previous image; restore backup data only if mutation incompatibility is observed.
- [ ] After any rollback, verify v4.5.1 health, three accounts and Gemini completion before closing the incident.
- [ ] Ensure Hermes aliases are absent/removed if their targets are not live.

**Dependencies:** TSK-003; execution branch conditional on TSK-011 failure.

**Definition of Done / evidence:** pre-deploy readiness is proven. If no rollback is triggered, mark the execution branch not applicable with the successful TSK-011 evidence; if triggered, provide complete post-rollback read-back.

**Requirements:** FR-9; AC-9.1, AC-9.2, AC-9.3; NFR-1, NFR-2, NFR-3, NFR-5.

### TSK-014. Run final QA audit and durable closeout

- [ ] Load the quality-assurance skill.
- [ ] Programmatically parse the coverage matrix and task statuses; no requirement may map only to an open/blocked task for a `PASS` verdict.
- [ ] Write `qa-report.md` with verdict, per-requirement evidence, tests, review result, owner action and remaining risks.
- [ ] Update the task checkboxes, Progress table and progress count from actual status.
- [ ] Update `/home/mint/hq/Projects/ag-tools.md` Status/Pending/Recent and append the final feature checkpoint.
- [ ] Commit/push only intended spec/evidence/HQ files as `mint <hutr8276@yandex.ru>`.

**Dependencies:** TSK-011, TSK-012 and the applicable branch of TSK-013.

**Definition of Done / evidence:** QA report has a grounded PASS or explicit FAIL/BLOCKED verdict; tasks and HQ agree with live state; commits/pushes verified.

**Requirements:** all requirements and acceptance criteria.

---

## Requirements coverage matrix

| Requirement | Covered by tasks |
|---|---|
| FR-1 | TSK-002, TSK-004, TSK-011 |
| FR-2 | TSK-003, TSK-004 |
| FR-3 | TSK-003, TSK-005, TSK-011 |
| FR-4 | TSK-006, TSK-011 |
| FR-5 | TSK-007, TSK-011 |
| FR-6 | TSK-003, TSK-006, TSK-011 |
| FR-7 | TSK-008, TSK-012 |
| FR-8 | TSK-009, TSK-010, TSK-011 |
| FR-9 | TSK-003, TSK-010, TSK-013 |
| NFR-1 | TSK-002–TSK-014 |
| NFR-2 | TSK-003–TSK-005, TSK-010, TSK-013 |
| NFR-3 | TSK-002–TSK-007, TSK-010, TSK-011, TSK-013, TSK-014 |
| NFR-4 | TSK-006, TSK-007, TSK-011 |
| NFR-5 | TSK-008, TSK-012, TSK-013 |

## Acceptance-criteria coverage matrix

| Acceptance criteria | Covered by tasks |
|---|---|
| AC-1.1 | TSK-002 |
| AC-1.2 | TSK-004, TSK-011 |
| AC-2.1 | TSK-003 |
| AC-2.2, AC-2.3 | TSK-004 |
| AC-3.1, AC-3.2 | TSK-003, TSK-005, TSK-011 |
| AC-3.3 | TSK-005, TSK-011 |
| AC-4.1, AC-4.2 | TSK-006, TSK-011 |
| AC-5.1, AC-5.2, AC-5.3, AC-5.4 | TSK-007, TSK-011 |
| AC-6.1 | TSK-003, TSK-006, TSK-011 |
| AC-6.2 | TSK-006, TSK-011 |
| AC-7.1, AC-7.3, AC-7.4 | TSK-008, TSK-012 |
| AC-7.2, AC-7.5 | TSK-012 |
| AC-8.1 | TSK-010 |
| AC-8.2 | TSK-011 |
| AC-9.1, AC-9.2 | TSK-003, TSK-010, TSK-013 |
| AC-9.3 | TSK-013 |

## Explicit gates

1. **No candidate runtime before TSK-002 and TSK-003 are green.**
2. **No owner Deploy handoff before TSK-007 and TSK-009 are green.**
3. **No production mutation by the agent.** The owner alone changes image and clicks Deploy in Dokploy.
4. **No Hermes alias activation before TSK-011 proves the six targets live.**
5. **Any account loss, missing model, entitlement error, signature error or Gemini regression blocks deployment or triggers rollback.**
