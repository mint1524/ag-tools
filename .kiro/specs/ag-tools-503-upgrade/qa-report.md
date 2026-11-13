# QA — AG Tools 503 / upstream 5.0.3
Дата: 2026-11-13. Вердикт: **UPGRADE PASS / GENERATION BLOCKED_EXTERNAL**.

## Проверенные границы
| Требование | Evidence / исполнение | Результат |
|---|---|---|
| FR-1 причина | baseline.md: real authenticated probes; app logs TOS_VIOLATION | PASS, Google403 а не outage |
| FR-2 сохранность | cold-backup-evidence.json: writer stopped, archive mode0600, restore exact file hashes,8JSON/4SQLite ok | PASS |
| FR-3 кандидат | shadow-evidence.json: pinned5.0.3, loopback-only, own volume, два старта; account/auth/5SQLite; JS/CSS200; noauth401 | PASS compatibility, не generation |
| FR-4 production | production-evidence.json + Dokploy exact image readback:5.0.3,1running, mounts/env/networks прежние; UI/assets200, catalog200, anonymous401 | PASS |
| FR-5 generation | Gemini403 account_forbidden/permission_error/TOS; Sonnet503 all_accounts_limited; no success | BLOCKED_EXTERNAL |
| FR-6 egress | owner explicit approval, shadow-direct-evidence.json и production pool_enabled=false; proxy list/bindings сохранены; network_error=false | PASS |
| NFR-1/2 | ready-made digest image, capped shadow; no Hermes/provider/DNS changes; secret-safe API allowlist, private backup, shadow stopped | PASS scoped |

## Принятие и review
Первый reviewer sa-0-4e506810 NO-GO до whole-system cold snapshot. Принято, каскад design/tasks: перед cutover scale0, дождаться остановки writer; реальное выполнение и restore PASS в cold-backup-evidence.json. Второй reviewer sa-0-5b8afba5 (deleg_d221cc5e) дал conditional GO после фактического cold proof, иных blockers не нашёл. Это выполнено до изменения config/Dokploy. Owner дал два отдельных явных допуска: production upgrade+rollback и proxy pool disable/direct IP exposure.

## Реальное исполнение
- Registry: docker.io/lbjlaq/antigravity-manager:v5.0.3@sha256:a10cc6e4580da081b84c86bdd715bd48e7867ad859d8adeecedf266203b70d51. Dokploy сохранённый образ прочитан после записи; deployment applicationStatus=done; Swarm и container image readback совпали.
- Cold archive: /home/gosha/backups/ag-tools/ag_tools_data-20261113T113808Z.tar.gz; sha25607e999d8bbe0ccc51ade4c0da6c14bc503b18f4de095b86361af8c6d2be611be; mode0600. Archive восстановлен, все file bytes совпали. Источник rollback snapshot ag_tools_backup_20261113T113808Z; независимая восстановленная копия ag_tools_restore_20261113T113808Z.
- После cutover: one indexed account; refresh token/API key/admin password/auth mode preserved; user_tokens count0 до/после; proxy entries/account_bindings не удалены; integrity ok security/proxy_logs/user_tokens/token_stats/thinking_store. Orphan credential files не активировали.
- Каталог14 exact IDs: новая версия capability-filtered; прежние79 catalog aliases не означали79 работающих моделей. Hermes catalog/config в этой задаче не трогали; старые clients могут показывать устаревшие aliases.
- Готовый бинарь, не собственная сборка: SOURCE test suite не запускалась и не заявляется. Проведены ISOLATED shadow и PUBLIC/PRODUCTION live checks. Владелец UI лично не просматривал; проверки HTTP/assets агентские.

## Ошибки/ограничения
См. review-resolution.md: первый online backup не whole-system point-in-time; начальные WAL/SHM/setup readiness/urljoin ошибки исправлены, последующие доказательства отдельные. Shadow stop timeout137 не OOM/graceful shutdown. В cutover между stop и readiness краткий502 ожидаемо наблюдался; финальный public health200/version5.0.3. Проверка rollback готовности основана на старом image+verified restored archive; фактический production rollback не выполнялся и не выдаётся за испытанный rollback drill.

## Rollback (только при миграционной/инфраструктурной регрессии)
Старый image ghcr.io/mint1524/ag-tools:sha-0066e12, локальный image ID sha256:18ecae6bf5e8a0e486a5d3daed47d6aeb33ae05e503ab2b9786f62ff34e7eb23 остаётся. Приватный исходный spec /home/gosha/backups/ag-tools/pre-503-upgrade-service-spec.json. При необходимости: scale только sidequests-agtools-a1xtnr=0, дождаться отсутствия writer, скопировать текущие migrated данные в отдельный quarantine volume, восстановить cold archive целиком в ag_tools_data (не поверх работающего writer, не оставляя новые DB/WAL); выбрать старый image через штатный dokploy-api.py set-image --image ghcr.io/mint1524/ag-tools:sha-0066e12, readback, deploy, затем health/auth/account checks. Не делать rollback только из-за внешнего Google ban: старый бинарь его также не снимает.

## Pending / unblock
TSK-006 остаётся внешним gate. Владелец подаёт апелляцию Google через UI/выданную upstream форму https://forms.gle/hGzM9MEUv2azZsrb9 либо добавляет собственный разрешённый аккаунт. Агент не подаёт апелляцию/не создаёт аккаунты и не обходит ограничения. После решения — штатный refresh и реальные Gemini+Claude smoke; только тогда можно заявлять генерацию восстановленной.

## Воспроизведение
- curl -fsS https://ai.okak.club/health → status ok/version5.0.3.
- python3 .kiro/specs/ag-tools-503-upgrade/dokploy-api.py read → exact pinned image/status done (значения credential не выводятся).
- ssh nlvmv2 python3 - < .kiro/specs/ag-tools-503-upgrade/production-check.py → scoped evidence; сейчас ожидаем Gemini/Sonnet refused, не generation PASS. Script compares frozen cold snapshot internally.
Не запускать cold-prep.py отдельно: это cutover template, его backup source embeds local approved runner; для повторного maintenance нужен свежий spec/backup и разрешение. shadow.py воспроизводит старый initial compatibility test, direct amendment доказан отдельно; не перезаписывать сохранённые финальные evidence диагностическими повторными запусками.
