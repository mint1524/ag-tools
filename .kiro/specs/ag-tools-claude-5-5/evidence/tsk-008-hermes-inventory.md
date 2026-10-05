# Evidence — TSK-008: Hermes OKAK profile inventory and activation plan

Дата проверки: 2026-10-05

## Inventory

Hermes recognizes 11 profiles/config homes. Exactly three already define `model_providers.okak` with `base_url=https://ai.okak.club/v1`:

| Profile | Config | Default | Approved 5.5 aliases now |
|---|---|---|---:|
| `default` | `/home/mint/.hermes/config.yaml` | `anthropic / claude-sonnet-5` | 0 |
| `study` | `/home/mint/.hermes/profiles/study/config.yaml` | `anthropic / claude-sonnet-5` | 0 |
| `work` | `/home/mint/.hermes/profiles/work/config.yaml` | `anthropic / claude-sonnet-5` | 0 |

Eight team profiles do **not** define an OKAK provider and therefore do not qualify:

```text
team-custodia
team-directio
team-facies
team-imaginatio
team-iudicium
team-nuntius
team-stabilis
team-substantia
```

These inherited configs already contain 90 historical `okak-*` alias keys despite lacking the provider; this is pre-existing state. The six new approved alias names are absent from all 11 profiles. The post-change negative assertion must compare the exact six keys and provider block against this baseline; it must not falsely claim the team profiles previously had zero historical OKAK-prefixed aliases.

No credential value, key-env name or auth-pool content was printed.

## Supported write mechanism proof

A scratch `HERMES_HOME` was used to test:

```bash
hermes config set \
  model_aliases.okak-claude-sonnet-5-5-low \
  '{"model":"claude-sonnet-5-5-low","provider":"okak"}'
```

The CLI exited 0 and serialized the exact mapping. The scratch home was then removed. No real profile was modified.

The installed launcher accepts `hermes -p <profile> ...` for both named profiles and `default`; read-only `config get model --json` was verified for `default`, `study` and `work`.

## Exact activation set

For each profile in `{default, study, work}`, run the following six writes **only after TSK-011 proves the production catalog live**:

```bash
hermes -p <profile> config set model_aliases.okak-claude-sonnet-5-5-low    '{"model":"claude-sonnet-5-5-low","provider":"okak"}'
hermes -p <profile> config set model_aliases.okak-claude-sonnet-5-5-medium '{"model":"claude-sonnet-5-5-medium","provider":"okak"}'
hermes -p <profile> config set model_aliases.okak-claude-sonnet-5-5-high   '{"model":"claude-sonnet-5-5-high","provider":"okak"}'
hermes -p <profile> config set model_aliases.okak-claude-opus-5-5-low      '{"model":"claude-opus-5-5-low","provider":"okak"}'
hermes -p <profile> config set model_aliases.okak-claude-opus-5-5-medium   '{"model":"claude-opus-5-5-medium","provider":"okak"}'
hermes -p <profile> config set model_aliases.okak-claude-opus-5-5-high     '{"model":"claude-opus-5-5-high","provider":"okak"}'
```

After writes, `hermes -p <profile> model --refresh` is the supported CLI cache-bust entrypoint (`--refresh` wipes `provider_models_cache.json` and re-fetches live `/v1/models`). Because it opens the interactive picker after refresh, activation will run it under a real PTY and cancel without changing the selected default; before/after `config get model --json` must match.

## Guard conditions

- Do not run the 18 writes while production v4.5.1 still serves 0/6 targets.
- Do not write any new provider or alias into the eight team profiles.
- Do not change the default model/provider in any qualified profile.
- Do not copy credentials across profiles.

## Verdict

**PASS for TSK-008 preparation.** Qualified set, negative baseline, unchanged defaults and exact supported commands are known. Alias activation itself remains TSK-012 and is blocked by the owner deployment plus successful live catalog proof.
