# Requirements — OKAK models in Hermes (micro-spec)

Status: implemented and verified; `qa-report.md` records the acceptance matrix and runtime-auth observation. Approved 2026-10-28 by «работай» for the initial plan, then expanded through explicit clarification for the common OKAK source and synchronized profiles. No production deployment is approved.

## Goal and scope
Restore the existing `https://ai.okak.club/v1` provider in Hermes. Initial scope was `work`; expanded explicitly on 2026-10-28 after discovering `hermes-provider-sync.timer` overwrote local changes from the root config every two minutes. Literal owner selection: «Да, исправить OKAK в общем источнике и синхронизируемых профилях».

Scope now includes only OKAK settings/aliases/catalogs in the root provider source and existing profiles with that exact endpoint. Preserve every profile's global default, every unrelated provider/credential, and the sync mechanism. This is configuration repair, not migration to OKAK AI Gateway or Claude 5.5 rollout.

## Requirements
- **FR-1** WHEN OKAK is selected without a model, Hermes SHALL select the live `gemini-3.8-flash-high` instead of the failing `gemini-3.6-flash-high`.
  - **AC-1.1** Given each eligible profile/source settings, when provider metadata is read after repair and after the next natural sync timer run, then OKAK default is exactly `gemini-3.8-flash-high`.
- **FR-2** WHEN the OKAK catalog is refreshed, configured `okak-*` aliases SHALL reference exact live catalog IDs, with no silent substitution from an old ID to another model. Aliases absent from discovery are removed from the configured picker/alias list, not remapped. This does not assert permanent retirement of those IDs.
  - **AC-2.1** Given authenticated live discovery, when config aliases are compared programmatically, then there is one alias per exact catalog ID with no stale aliases or collisions. Existing aliases for retained IDs are preserved.
  - **AC-2.2** Given the normal desktop model-options builder in work, when the OKAK row is read without a global refresh, then its model IDs exactly match live discovery, not only the declared default.
- **FR-3** WHEN a real Hermes session uses the repaired provider, Hermes SHALL receive a final model answer and complete a harmless tool-call/result loop over the normal streaming transport.
  - **AC-3.1** Given a fresh Hermes run using configured OKAK, when asked to read a fixture, then its transcript records a real file tool result followed by a final answer containing the fixture marker. Verify Gemini High and Sonnet 4.6 separately, and preserve provider/base URL/model provenance.
- **NFR-1** Before repair, save mode-0600 backups and sanitized before-states for all eligible homes. After repair, programmatically assert that only `providers.okak.default_model`, explicit `providers.okak.api_mode` matching the existing `openai_chat` transport, and OKAK aliases changed; global `model`, other providers and aliases, credential files remain unchanged by the configuration repair. Background/runtime auth-metadata maintenance over the longer session is recorded separately and is not rolled back.
- **NFR-2** No production deploy/restart/image/volume/account change, no writes outside the approved eligible profile/source OKAK settings and catalogs, no secret values in git/HQ/evidence/chat. Existing desktop sessions are not forced to switch models.

## Acceptance boundary
Direct API probes are diagnostic evidence, not Hermes acceptance. Live catalog membership alone is not a promise that every catalog ID is usable for agent work (e.g. image models); the complete agent cycle is verified on the two representative chat models above.
