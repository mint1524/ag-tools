---
feature: ag-tools-claude-5-5
phase: requirements
status: approved
approved_by: owner
approved_at: 2026-10-05
---

# Requirements: AG Tools → Claude 5.5

## 1. Overview

Обновить действующий личный провайдер OKAK (`https://ai.okak.club/v1`, AG Tools) с форка на базе Antigravity Tools v4.5.1 до чистого upstream v4.9.4, чтобы шесть вариантов Claude Sonnet 5.5 и Claude Opus 5.5 были доступны через OpenAI-compatible API и во всех Hermes-профилях, где уже настроен провайдер `okak`.

Продакшен-деплой остаётся ручным действием владельца в Dokploy. Агент готовит и проверяет кандидат на копии данных, но не нажимает Deploy.

## 2. Baseline, подтверждённый 2026-10-05

- Dokploy application: `ag-tools`, service `sidequests-agtools-a1xtnr`, `1/1` replica.
- Runtime image: `ghcr.io/mint1524/ag-tools:sha-0066e12`.
- `/health`: HTTP 200, version `4.5.1`.
- Пул: 3 аккаунта, все Google AI Pro, `disabled=0`, `proxy_disabled=0`.
- Живой `gemini-3.8-flash-low` completion: HTTP 200, ответ `OK`.
- `/v1/models`: 91 модель; Claude 5.5 отсутствует.
- Целевой immutable image: `docker.io/lbjlaq/antigravity-manager:v4.9.4@sha256:eef6a4d326d420429b3a31b9f3cc1a990fcc462704afd9aa2d58d74224cf1e47`.

## 3. User stories

### US-1 — новые модели

Как владелец OKAK, я хочу использовать Claude Sonnet 5.5 и Claude Opus 5.5 через существующий провайдер `okak`, чтобы новые модели были доступны без отдельного API-провайдера и без изменения клиентского endpoint.

### US-2 — безопасное обновление

Как оператор production, я хочу проверить новую версию на копии данных и иметь полный rollback, чтобы обновление не потеряло три живых Google-аккаунта и не сломало существующие Gemini-запросы.

### US-3 — Hermes discovery

Как пользователь Hermes, я хочу видеть шесть Claude 5.5 вариантов во всех профилях, где уже настроен OKAK, чтобы выбирать их через стабильные `okak-*` aliases.

## 4. Functional requirements

### FR-1. Immutable upstream candidate

WHEN кандидат подготавливается THEN система SHALL использовать чистый upstream Antigravity Tools v4.9.4, закреплённый по digest `sha256:eef6a4d326d420429b3a31b9f3cc1a990fcc462704afd9aa2d58d74224cf1e47`.

- AC-1.1: Given registry access, when digest read-back выполняется, then он совпадает с утверждённым digest.
- AC-1.2: Given candidate runtime, when `/health` вызывается, then response is HTTP 200 and version is `4.9.4`.

### FR-2. Backup and isolation

WHEN начинается подготовка кандидата THEN система SHALL создать проверенный timestamped backup production volume до запуска новой версии.

- AC-2.1: Given текущий live volume, when backup создан, then архив существует, имеет ненулевой размер и проходит проверку содержимого.
- AC-2.2: Given shadow runtime, when он запускается, then он использует отдельную копию данных и не монтирует live volume.
- AC-2.3: Given shadow runtime, when его порт публикуется, then он доступен только через loopback и не подключён к production Traefik.

### FR-3. Data compatibility

WHEN v4.9.4 открывает копию данных v4.5.1 THEN система SHALL сохранить работоспособность трёх Google-аккаунтов.

- AC-3.1: Given shadow startup, when account index читается, then account count equals 3.
- AC-3.2: Given migrated copy, when account states читаются, then `disabled=0`, `proxy_disabled=0`, providers contain only Google.
- AC-3.3: Given candidate restarted a second time, when health and account API are checked, then the same three accounts remain readable.

### FR-4. Claude 5.5 catalog

WHEN клиент запрашивает `/v1/models` у кандидата THEN система SHALL возвращать все шесть утверждённых ID:

1. `claude-sonnet-5-5-low`
2. `claude-sonnet-5-5-medium`
3. `claude-sonnet-5-5-high`
4. `claude-opus-5-5-low`
5. `claude-opus-5-5-medium`
6. `claude-opus-5-5-high`

- AC-4.1: Given valid proxy credential, when `/v1/models` is called, then all six IDs are present exactly once.
- AC-4.2: Given the returned catalog, when IDs are compared to the approved set, then there are no missing approved IDs.

### FR-5. Claude 5.5 generation

WHEN короткий smoke-запрос отправляется на каждый утверждённый ID THEN система SHALL вернуть успешный OpenAI-compatible completion либо явно классифицированный внешний entitlement blocker.

- AC-5.1: Given eligible Google account, when each of six models receives a bounded single-turn request, then each returns HTTP 200 and non-empty assistant content.
- AC-5.2: Given `claude-sonnet-5-5-high`, when a second conversational turn is sent, then it returns HTTP 200 without `thinking.signature` failure.
- AC-5.3: Given `claude-opus-5-5-high`, when a second conversational turn is sent, then it returns HTTP 200 without `thinking.signature` failure.
- AC-5.4: Given account entitlement is absent, when upstream returns 403/404, then production deployment remains blocked and the exact model/account eligibility is reported without exposing tokens.

### FR-6. Existing Gemini regression safety

WHEN candidate verification completes THEN существующий Gemini path SHALL продолжать работать.

- AC-6.1: Given current proxy credential, when `gemini-3.8-flash-low` receives the bounded `Reply exactly: OK` request, then response is HTTP 200 with assistant content `OK`.
- AC-6.2: Given candidate `/v1/models`, when compared with the current required Gemini model, then `gemini-3.8-flash-low` remains present.

### FR-7. Hermes OKAK aliases

WHEN backend candidate прошёл shadow verification THEN каждый Hermes-профиль, уже содержащий provider `okak`, SHALL получить шесть стабильных aliases `okak-<model-id>`.

- AC-7.1: Given profile inventory, when profiles are classified, then only profiles with an existing `okak` provider are selected for change.
- AC-7.2: Given a selected profile, when aliases are read back, then all six map to exact `{provider: okak, model: <approved-id>}` values.
- AC-7.3: Given an unselected profile without OKAK, when configuration is read back, then no new OKAK provider or alias has been introduced.
- AC-7.4: Given alias update, when current default model is read back, then it is unchanged.
- AC-7.5: Given refreshed provider cache, when the model picker resolves OKAK, then all six live IDs are discoverable.

### FR-8. Manual production gate

WHEN candidate, backup, shadow tests and alias verification are complete THEN система SHALL остановиться перед production deployment и выдать владельцу точные Dokploy steps.

- AC-8.1: Given agent access to Dokploy API, when preparation completes, then no deploy/redeploy/update image call has been made by the agent.
- AC-8.2: Given owner deploy confirmation, when post-deploy verification starts, then actual runtime image, replicas, health, accounts, model catalog and bounded generation are read back before success is declared.

### FR-9. Rollback

WHEN post-deploy verification fails THEN система SHALL provide a rollback to the previous immutable image and, if necessary, the pre-deploy volume backup.

- AC-9.1: Given failure after owner deployment, when rollback instructions are issued, then they identify the exact previous image `ghcr.io/mint1524/ag-tools:sha-0066e12`.
- AC-9.2: Given data incompatibility, when rollback is executed, then the pre-deploy backup is the restoration source rather than the potentially migrated live volume.
- AC-9.3: Given rollback completion, when checks run, then health is 200, account count is 3 and Gemini smoke passes.

## 5. Non-functional requirements

### NFR-1. Secrets

- Tokens, account emails, refresh tokens, admin passwords and API keys SHALL NOT be printed, committed, copied to HQ or included in reports.
- Presence checks SHALL return booleans/counts only.

### NFR-2. Production safety

- The agent SHALL NOT invoke production deploy/redeploy/update-image actions.
- The live service SHALL remain running during backup and shadow verification.
- Managed services SHALL NOT be killed manually.

### NFR-3. Reproducibility

- Every asserted image, model count and account count SHALL have command/API evidence.
- The target image SHALL be referenced by immutable digest, not only by mutable tag.

### NFR-4. Bounded quota use

- Smoke prompts SHALL be minimal and use bounded output.
- No load or soak test is included.

### NFR-5. Profile isolation

- Profile changes SHALL use each profile’s own config/credential boundary.
- Credentials SHALL NOT be copied between profiles.

## 6. Out of scope

- Сохранение или перенос самописного ChatGPT/Codex provider-кода форка.
- Разработка нового OKAK AI Gateway.
- Исправление HTTP `proxy_pool` 7/7 health-check failures.
- Изменение default model, тарифов или маршрутизации других providers.
- Создание OKAK provider в профилях, где его ещё нет.
- Автоматический production deployment.

## 7. Owner decisions — 2026-10-05

1. Использовать чистый upstream v4.9.4; наш ChatGPT/Codex-код больше не сохранять.
2. Экспонировать все шесть Sonnet/Opus 5.5 low/medium/high IDs.
3. Обновить все Hermes-профили, где уже настроен OKAK.
4. Агент готовит и shadow-проверяет кандидат; владелец вручную нажимает Deploy в Dokploy.

## 8. Risks and known pending items

- Upstream v4.9.4 выпущен 2026-10-04 и требует реального shadow/multi-turn proof.
- Старый v4.9.1 имел deterministic multi-turn `thinking.signature` bug; v4.9.4 должен доказать исправление.
- Все три аккаунта идентифицированы как Google AI Pro, но non-trial entitlement будет доказан только реальным 5.5 smoke.
- Старый JSON corruption writer остаётся в production v4.5.1 до owner deployment; upstream исправил этот класс в v4.6.3.
- HTTP proxy pool failures остаются отдельным pending и не блокируют эту миграцию, если прямой upstream path зелёный.
