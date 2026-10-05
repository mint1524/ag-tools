---
feature: ag-tools-claude-5-5
verdict: BLOCKED
updated: 2026-10-05
owner_deploy: not_performed
---

# QA report: AG Tools → Claude 5.5

## Verdict

**BLOCKED — production handoff is intentionally closed.**

The v4.9.4 artifact, backup, isolation, data migration and Gemini regression checks passed. The defining feature gate did not: after a real quota refresh, all three copied Google AI Pro accounts expose 29 upstream quota models and none exposes Claude 5.5. Shadow `/v1/models` returns 37 IDs and 0/6 approved IDs. A bounded Sonnet 5.5 probe is returned to the client as HTTP 503; secret-safe shadow logs prove upstream HTTP 404 for all three accounts on both production and sandbox endpoints.

This is the explicit FR-5 / AC-5.4 entitlement-capability branch. It is not safe to advertise aliases or deploy a backend that still cannot serve the requested models.

## Execution state

| Gate | Verdict | Evidence |
|---|---|---|
| Immutable v4.9.4 input | PASS | `evidence/tsk-002-supply-chain.md` |
| Production baseline and restore-proof backup | PASS | `evidence/tsk-003-baseline-backup.md` |
| Shadow isolation and health | PASS | `evidence/tsk-004-shadow-isolation.md` |
| v4.5.1 data compatibility across two starts | PASS | `evidence/tsk-005-data-compatibility.md` |
| Claude 5.5 live catalog | **FAIL/BLOCKED** | `evidence/tsk-006-007-capability-blocker.md` — approved 0/6 |
| Existing Gemini catalog and completion | PASS | same evidence — model present; HTTP 200, content `OK` |
| Claude 5.5 single-turn | **FAIL/BLOCKED** | one bounded Sonnet-high probe; upstream 404 for all three accounts |
| Claude 5.5 multi-turn | BLOCKED | single-turn prerequisite is not available |
| Hermes profile inventory | PASS | `evidence/tsk-008-hermes-inventory.md` |
| Hermes alias activation | NOT RUN / BLOCKED | backend does not serve the targets |
| Rollback readiness | PASS for pre-deploy branch | `evidence/tsk-013-rollback-readiness.md` |
| Production deployment | NOT RUN | explicit owner-only gate; no handoff issued |

## Requirements audit

Mechanical parsing of `tasks.md` produced:

```text
Done: 6
Blocked: 6
In progress: 1
Ready / conditional: 1
Requirements with no completed covering task: FR-4, FR-5, FR-8, NFR-4
```

| Requirement group | Status | Reason |
|---|---|---|
| FR-1 | Partial PASS | Digest and shadow health pass; production remains intentionally on v4.5.1. |
| FR-2 | PASS | Verified backup, dedicated volume, loopback bind and no Traefik/Dokploy ownership. |
| FR-3 | PASS in shadow | Three accounts survive first start and restart; production migration was not attempted. |
| FR-4 | **FAIL/BLOCKED** | Live candidate catalog contains 0/6 approved IDs. |
| FR-5 | **BLOCKED by AC-5.4** | Upstream account capability omits 5.5 and direct probe receives upstream 404. |
| FR-6 | PASS | Required Gemini model remains present and bounded completion returns `OK`. |
| FR-7 | Partial PASS | Qualified profiles and exact mappings are prepared; aliases correctly remain absent. |
| FR-8 | **BLOCKED** | Candidate is not deployable, so owner Deploy handoff is not issued. |
| FR-9 | PASS for readiness | Previous image and verified backup exist; rollback execution was not triggered. |
| NFR-1 | PASS | Reports contain only counts/status/classification; no secret/account identifiers. |
| NFR-2 | PASS | Production remained running; no Deploy/redeploy/update-image call; disposable shadow stopped cleanly. |
| NFR-3 | PASS | Immutable refs, counts and runtime facts have command/API evidence. |
| NFR-4 | Partial | Requests were bounded, but successful Claude quota proof is impossible while capability is absent. |
| NFR-5 | PASS so far | No profile credential copied and no profile configuration changed. |

## Acceptance-criteria result

- PASS: AC-1.1; AC-1.2 in shadow; AC-2.1–AC-2.3; AC-3.1–AC-3.3 in shadow; AC-5.4; AC-6.1–AC-6.2; AC-7.1, AC-7.3; AC-8.1.
- FAIL/BLOCKED: AC-4.1–AC-4.2; AC-5.1–AC-5.3; AC-7.2, AC-7.4–AC-7.5; AC-8.2.
- READY / NOT TRIGGERED: AC-9.1–AC-9.3. Previous image and backup source are proven, but there was no owner deployment, data incompatibility or rollback execution.
- Production-scoped repetitions of health/account/model checks remain blocked until an owner deployment is justified and performed.

## Production and disposable state

- Production read-back after shadow shutdown: one running task, image `ghcr.io/mint1524/ag-tools:sha-0066e12`, public `/health` HTTP 200.
- Candidate container `ag-tools-shadow-20261005`: cleanly stopped, status `exited`.
- Shadow volume and verified backup volume remain for a bounded retry.
- Backup archive remains mode 0600 and its checksum was re-verified.
- No Dokploy mutation was performed.
- No Hermes aliases were activated.

## Independent review

An approved `claude-sonnet-5` / high read-only review is in progress against the spec, evidence and source lineage. The reviewer was explicitly prohibited from production mutations, shared-tree changes and secret/config-value access. This QA report does not upgrade the gate while the review is pending.

## Exact unblock condition

1. At least one copied account’s real quota refresh exposes all six approved Claude 5.5 variants, or a separately confirmed eligible Google AI Pro non-trial/Ultra account is added by the owner.
2. Restart the preserved shadow candidate, refresh quota and assert catalog 6/6.
3. Run six bounded single-turn smokes and two high multi-turn smokes with zero `thinking.signature` failures.
4. Obtain a clean independent review with no open suggestions.
5. Only then issue the manual Dokploy owner guide. After owner Deploy, repeat live production checks before adding aliases to `default`, `study` and `work`.

## Owner state

The owner authorized execution of the approved plan. The owner has **not** deployed v4.9.4; deployment is correctly unnecessary and unsafe while this external capability gate is red.
