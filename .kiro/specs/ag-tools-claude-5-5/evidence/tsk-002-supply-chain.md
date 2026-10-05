# Evidence — TSK-002: immutable candidate and upstream capability

Дата проверки: 2026-10-05
Target ref: `v4.9.4` / `277c0062480d14031757c5a6052ef27b3d57c39f`

## Registry identity

- `docker buildx imagetools inspect docker.io/lbjlaq/antigravity-manager:v4.9.4` returned manifest digest:
  `sha256:eef6a4d326d420429b3a31b9f3cc1a990fcc462704afd9aa2d58d74224cf1e47`.
- `nlvmv2` pulled by the immutable reference, not by tag only.
- Remote `docker image inspect` read-back:
  - image ID: `sha256:eef6a4d326d420429b3a31b9f3cc1a990fcc462704afd9aa2d58d74224cf1e47`;
  - repo digest: `lbjlaq/antigravity-manager@sha256:eef6a4d326d420429b3a31b9f3cc1a990fcc462704afd9aa2d58d74224cf1e47`;
  - unpacked image size reported by Docker: `439201517` bytes.
- Free space after pull: 9.4 GiB on `/var/lib/docker` filesystem. No image build was run.

## Official model set at the immutable source ref

`src-tauri/resources/official_models.json` was parsed from the target commit. Exact matching keys:

```text
claude-opus-5-5-high
claude-opus-5-5-low
claude-opus-5-5-medium
claude-sonnet-5-5-high
claude-sonnet-5-5-low
claude-sonnet-5-5-medium
```

The assertion compared the sorted result to the approved six-ID set and exited 0. `src-tauri/src/proxy/model_specs.rs` also contains Claude 5.5 family handling at the same ref.

## Fix lineage included in v4.9.4

Each commit below passed `git merge-base --is-ancestor <commit> 277c006...`:

- `af5b791ba99c41adce7d99c2810f50e539114910` — align official Claude 5.5 catalog and dynamic capability handling.
- `11cf588a19f92d874060ac208ee79106d1a64e26` — fix Claude 5.5 raw Protobuf thought-signature recognition and outbound format validation.
- `083039d2434e6b3c28e4e3b5b360d644d9adf651` — add Claude 5.5 signature-family validation.
- `66ff1328b5df2791a07b64751b0f1956523c7cbd` — self-healing account JSON parser and per-account write lock.
- `6a665a9197561a4db0260fd16ad5b66417c1bf3c` — self-heal corrupted account index.

`git describe --tags 277c006...` returned `v4.9.4`.

## Data-shape compatibility hypothesis

At the exact target ref, concatenated source for:

- `src-tauri/src/models/account.rs`
- `src-tauri/src/proxy/config.rs`

contained zero `deny_unknown_fields` occurrences. This supports the design hypothesis that unknown fork-added JSON fields are ignored by Serde. It is not accepted as runtime proof; TSK-005 must still verify first start and restart against a copied production dataset.

## Verdict

**PASS for TSK-002.** The approved tag, source commit, registry digest, local remote-host image and six model IDs are mutually consistent. Runtime/data compatibility remains gated by TSK-003–TSK-007.
