# Tasks — ag-tools-503-upgrade
Краткая ops-spec. Разрешение owner production override: 2026-11-13, exact quote в requirements.

- [x] **TSK-001** — baseline/root cause и pinned stable release. FR-1,NFR-1. Evidence: baseline.md. Health200/v4.5.1; каталог79; два429, точный upstream403 TOS_VIOLATION; registry manifest подтверждён.
- [x] **TSK-002** — private backup+restore-proof и frozen rollback handles. FR-2,NFR-1/2. Depends001. Deliverable backup-evidence.json; acceptance all JSON/SQLite valid, hashes/config/account identity match.
- [x] **TSK-003** — pinned isolated shadow, два старта, integrity/auth/catalog/UI/error-path. FR-3,NFR-1/2. Depends002. Deliverable shadow-evidence.json; acceptance health5.0.3, no production mount/public bind, preserved clients/accounts; ban marked external.
- [x] **TSK-004** — review gates, cold cutover backup после остановки writer, разрешённое отключение proxy_pool, Dokploy update+deploy и exact readback. FR-4/6,NFR-1/2. Depends003. Deliverable production-evidence.json; acceptance stored/runtime digest agree, service scope preserved, health/UI/auth/data pass; conditional rollback.
- [x] **TSK-005** — QA, Progress, HQ handoff с отдельным generation gate. FR-1..6,NFR-1/2. Depends004. Deliverable qa-report.md; acceptance no unjustified PASS.
- [ ] **TSK-006** — восстановить generation после owner Google action. FR-5. Depends005 + external Google unban/eligible account. BLOCKED_EXTERNAL; no account added or appeal sent by agent.

## Progress
| Complete | In progress | Pending | Blocked external | Total |
|---|---|---|---|---|
| 5 | 0 | 0 | 1 | 6 |
