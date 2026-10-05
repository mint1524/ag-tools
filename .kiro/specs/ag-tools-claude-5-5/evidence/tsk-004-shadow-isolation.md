# Evidence — TSK-004: isolated v4.9.4 shadow

Дата проверки: 2026-10-05

## Shadow resources

- source snapshot volume: `ag_tools_backup_20261005T175800Z` (read-only during clone);
- shadow volume: `ag_tools_shadow_20261005T175800Z`;
- container: `ag-tools-shadow-20261005`;
- container ID: `8f01eb0614814eed2ce221c2f649a74c41c7aa3b03153536c180ca5a264aeb94`;
- image ID: `sha256:eef6a4d326d420429b3a31b9f3cc1a990fcc462704afd9aa2d58d74224cf1e47`;
- network: standalone Docker `bridge`;
- restart policy: `no`.

## Isolation read-back

`docker inspect` assertions passed:

```text
mount: ag_tools_shadow_20261005T175800Z -> /root/.antigravity_tools (RW)
production volume ag_tools_data mounted: false
port binding: 127.0.0.1:18045 -> 8045/tcp
Traefik labels: 0
Dokploy labels: 0
public/non-loopback health reachability: false
```

The container is not a Swarm/Dokploy service and cannot receive traffic through the production domain.

## Readiness

```text
GET http://127.0.0.1:18045/health
HTTP 200
{"status":"ok","version":"4.9.4"}
```

## Bounded log safety check

The first 8,813 log bytes were analyzed in-memory without printing lines or account data:

- email-like patterns: 0;
- `thinking.signature`/invalid-signature errors: 0;
- panic/fatal markers: 0;
- secret-field markers: 1 generic `api key:` message; the suffix had no assignment and its longest token was 3 characters, so no key value was present.

## Production isolation

The production Swarm service was not restarted or updated. No Dokploy mutation API was called.

## Verdict

**PASS for TSK-004.** The pinned candidate is healthy, loopback-only, independent of the live volume and outside Traefik/Dokploy ownership.
