# Evidence — TSK-005: v4.5.1 data compatibility on v4.9.4

Дата проверки: 2026-10-05
Shadow: `ag-tools-shadow-20261005` / volume `ag_tools_shadow_20261005T175800Z`

## First startup

The copied dataset was inspected both on disk and through the authenticated v4.9.4 admin API. The admin password was discovered inside the shadow-only `gui_config.json`, used in-memory, and never printed.

```text
health: HTTP 200, version 4.9.4
file_index_count=3
file_account_count=3
file_malformed_count=0
file_disabled_count=0
file_proxy_disabled_count=0
file_provider_set=["google"]
corrupt_index_backup_count=0
account_api_status=200
account_api_count=3
account_api_disabled_count=0
account_api_proxy_disabled_count=0
account_api_provider_set=["google"]
```

## Clean restart

The disposable standalone container was stopped with `docker stop --time 30` and returned `exited`. The same container was started again; no Swarm/Dokploy service was touched.

```text
restart_health_status=200
restart_health_version=4.9.4
same_container_id=true
restart_status=running
restart_count=0
image_id=sha256:eef6a4d326d420429b3a31b9f3cc1a990fcc462704afd9aa2d58d74224cf1e47
```

Second-start file/API invariants were identical:

```text
file_index_count=3
file_account_count=3
file_malformed_count=0
file_disabled_count=0
file_proxy_disabled_count=0
file_provider_set=["google"]
corrupt_index_backup_count=0
account_api_status=200
account_api_count=3
account_api_disabled_count=0
account_api_proxy_disabled_count=0
account_api_provider_set=["google"]
```

## Verdict

**PASS for TSK-005.** All three v4.5.1 Google account files and the index are accepted by v4.9.4; no account is lost, disabled, proxy-disabled, malformed or quarantined after first startup and clean restart.
