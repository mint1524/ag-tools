# Evidence — TSK-013: rollback readiness

Дата проверки: 2026-10-05

## Read-back

- Backup archive exists: `/home/gosha/backups/ag-tools/ag_tools_data-20261005T175800Z.tar.gz`.
- Archive mode: `0600`.
- SHA-256 still equals `12e54182bf30fcc4df71310b9a1277982acc1fedf3ba61a878b711c9f91f80b2`.
- Snapshot volume `ag_tools_backup_20261005T175800Z` still exists.
- Previous image `ghcr.io/mint1524/ag-tools:sha-0066e12` is still local as image ID `sha256:18ecae6bf5e8a0e486a5d3daed47d6aeb33ae05e503ab2b9786f62ff34e7eb23`.
- Production service is still configured to that previous image and has one running task.

## Conditional rollback procedure

No rollback is currently needed because no production deployment occurred. If a later owner deployment fails verification:

1. In Dokploy application `sidequests/agtools`, restore Docker image `ghcr.io/mint1524/ag-tools:sha-0066e12`; leave volume/domain/port/environment unchanged; click Deploy.
2. If and only if v4.9.4 incompatibly mutated the live data, restore the verified archive/snapshot into `ag_tools_data` while the owner-controlled service is stopped by the normal platform workflow. Do not overwrite live data merely because a request failed.
3. Verify v4.5.1 health, three Google accounts with zero disabled/proxy-disabled, and `gemini-3.8-flash-low` HTTP 200/`OK`.
4. Do not install the six aliases; if they were activated after a later deployment, remove only those exact six keys when their targets are absent.

## Status

**READY / not executed.** The rollback inputs are verified. The execution branch remains conditional on a future owner deployment and post-deploy failure.
