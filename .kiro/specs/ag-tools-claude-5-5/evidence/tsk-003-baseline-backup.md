# Evidence — TSK-003: live baseline and verified backup

Дата проверки: 2026-10-05

## Live baseline before backup

Swarm/Docker read-back on `nlvmv2`:

- service: `sidequests-agtools-a1xtnr`;
- configured image: `ghcr.io/mint1524/ag-tools:sha-0066e12`;
- desired replicas: 1; running tasks: 1; running containers: 1;
- service update state: `completed`;
- container status: `running`; started `2026-09-25T16:27:04.394197399Z`; restart count 0;
- data mount: Docker volume `ag_tools_data` → `/root/.antigravity_tools`, RW;
- container image ID: `sha256:18ecae6bf5e8a0e486a5d3daed47d6aeb33ae05e503ab2b9786f62ff34e7eb23`.

Secret-safe data read through a disposable read-only mount:

```text
account_index_count=3
account_file_count=3
account_disabled_count=0
account_proxy_disabled_count=0
account_provider_set=["google"]
```

Live public API read-back through the existing work-profile credential:

```text
health_status=200
health_version=4.5.1
models_status=200
models_count=91
claude_5_5_count=0
gemini_status=200
gemini_content_ok=true
```

The key was resolved from the profile-owned `key_env`; its name/value were not printed.

## Backup method

Artifact:

- snapshot volume: `ag_tools_backup_20261005T175800Z`;
- archive: `/home/gosha/backups/ag-tools/ag_tools_data-20261005T175800Z.tar.gz`;
- archive size: `63523222` bytes;
- SHA-256: `12e54182bf30fcc4df71310b9a1277982acc1fedf3ba61a878b711c9f91f80b2`;
- mode/owner: `0600`, `gosha`; parent directory mode `0700`.

JSON/log/config files were copied to a new Docker volume. Each live SQLite database was copied with Python `sqlite3.Connection.backup()` while the production application stayed running; this provides SQLite’s online consistent backup semantics. The source mount was writable only because SQLite needs its normal lock/shared-memory path during an online backup; no application records were changed by the backup routine.

The account JSON set was hashed internally immediately before and after copying. Only the boolean comparison was emitted: `source_account_json_stable=true`.

SQLite copy results:

```text
proxy_logs.db integrity=ok
security.db integrity=ok
token_stats.db integrity=ok
user_tokens.db integrity=ok
```

Snapshot invariants:

```text
backup_account_index_count=3
backup_account_file_count=3
backup_disabled_count=0
backup_proxy_disabled_count=0
backup_provider_set=["google"]
```

## Restore verification

The compressed archive was extracted into a fresh disposable Docker volume. The restored copy independently produced:

- account index/file counts: 3 / 3;
- disabled/proxy-disabled: 0 / 0;
- provider set: Google only;
- `PRAGMA integrity_check=ok` for all four SQLite databases.

The restore-check volume was removed after verification. The durable snapshot volume and archive remain for shadow startup and rollback.

## Diagnostic corrected during the task

A first backup attempt mounted the live source read-only. `proxy_logs.db` and `user_tokens.db` opened, but `security.db` and `token_stats.db` could not open because SQLite needed its lock/shared-memory path. The incomplete snapshot volume was identified and removed; zero incomplete `ag_tools_backup_*` volumes remained before the successful run. The successful backup used SQLite’s online backup API with normal locking and was then restored/verified.

## Production-safety result

- Production service stayed running throughout.
- No Dokploy deploy/redeploy/update-image call was made.
- After backup, Swarm still reported the old configured image and one Running task.

## Verdict

**PASS for TSK-003.** A verified, restorable, secret-safe pre-migration backup exists, and the live baseline is recorded.
