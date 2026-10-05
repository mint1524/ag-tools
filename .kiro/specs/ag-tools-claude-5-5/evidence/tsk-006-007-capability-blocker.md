# Evidence — TSK-006/007 blocker: live account capability lacks Claude 5.5

Дата проверки: 2026-10-05
Shadow image: v4.9.4 digest `sha256:eef6a4d326d420429b3a31b9f3cc1a990fcc462704afd9aa2d58d74224cf1e47`

## Tight-loop result

Authenticated shadow `GET /v1/models` returned HTTP 200 and 37 models, but:

```text
approved_present_count=0
approved_duplicate_count=0
approved_missing=[all six approved Sonnet/Opus 5.5 IDs]
gemini_model_present=true
```

The runtime list includes Claude 4.6 and current Gemini families but no Claude 5.5.

## Root cause investigation

### 1. The image/source does contain 5.5 support

TSK-002 proved that the exact v4.9.4 source/digest embeds the official six-ID catalog and Claude 5.5 signature-family fixes. Therefore the absence is not caused by accidentally running v4.5.1 or the wrong image.

### 2. `/v1/models` is account-capability driven

At v4.9.4, `handlers/openai.rs::handle_list_models` calls `get_all_dynamic_models`. `model_mapping.rs` first gathers `TokenManager::get_all_collected_models()` from each account’s `quota.models`; the hardcoded baseline adds Claude 4.6 but does not unconditionally advertise 5.5. `token_manager.rs` includes a dedicated test proving that a PRO account without 5.5 in refreshed quota is filtered from 5.5 routing.

### 3. All three copied production accounts lack the capability

Before quota refresh, each of the three PRO accounts had 29 quota models and zero Claude 5.5 entries. A real authenticated `POST /api/accounts/refresh` returned HTTP 200; afterward all three still had exactly 29 quota models and zero Claude 5.5 entries. Their model-set union and intersection are both 29 and contain Claude 4.6/Gemini families only.

No trial/non-trial flag is present in saved metadata; the only allowlisted plan field is `quota.subscription_tier=PRO` for all three accounts. This is insufficient to claim they are non-trial or included in Google’s rollout.

### 4. Direct 5.5 request proves upstream rejection

A bounded `claude-sonnet-5-5-high` request was sent after the catalog failure. The client-facing response was HTTP 503 / `all_accounts_limited`, but bounded, redacted candidate logs show the actual attempts:

- request normalized to real model `claude-sonnet-5-5-high`;
- each account was tried;
- Google `daily-cloudcode-pa` production and sandbox `streamGenerateContent` endpoints returned HTTP 404 for each attempt;
- repeated attempts only cycled a short per-account cooldown; they did not produce a success.

Thus the client-facing 503 is a collapsed scheduler result; the underlying capability failure is upstream 404 across all three accounts.

## Gemini regression proof

Despite the blocked 5.5 capability:

```text
shadow_gemini_status=200
shadow_gemini_nonempty=true
shadow_gemini_exact_ok=true
```

`gemini-3.8-flash-low` remains present and functional on v4.9.4.

## Safety outcome

- No production deployment occurred.
- No Hermes alias was installed that would point at unavailable models.
- The shadow remains loopback-only for follow-up refresh/retest.
- The durable pre-migration backup remains unchanged.

## Verdict

- **TSK-006: BLOCKED / partial.** Gemini regression passes, but catalog acceptance AC-4.1/AC-4.2 fails (0/6).
- **TSK-007: BLOCKED.** Six-model and multi-turn verification cannot honestly pass while upstream returns 404 and the runtime catalog omits all six IDs.
- **TSK-010 production gate: CLOSED.** Deploying v4.9.4 now would not deliver the user-requested models.

This is an external account rollout/entitlement blocker, not a reason to add fake static aliases or deploy an unverified candidate.
