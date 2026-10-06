# Evidence — TSK-007 follow-up: why the three accounts have no Claude 5.5

Дата: 2026-10-06. Read-only относительно production; использовалась только shadow-копия.

## Прямой ответ Google (не AG Tools)

Прямые `loadCodeAssist` и `fetchAvailableModels` (daily-cloudcode-pa, свежие токены после refresh), одинаково для всех трёх аккаунтов:

- `paidTier = g1-pro-tier` («Google AI Pro»); `currentTier = standard-tier`.
- `ineligibleTiers = [free-tier: UNSUPPORTED_LOCATION]`; `gcpManaged=false`.
- `fetchAvailableModels`: 33 модели; из Claude/GPT только `claude-opus-4-6-thinking`, `claude-sonnet-4-6`, `gpt-oss-120b-medium`; **ни одной 5.5**.
- `agentModelSorts` («Recommended») и `tieredModelIds` тоже не содержат 5.5.
- Ответ не зависит от User-Agent (app UA, hub/2.17, antigravity/5.0, hub/2.18 — одинаковые 33 модели). Без UA — 403.

## Официальная документация (antigravity.google/docs/models, прочитано в браузере)

Claude Sonnet 5.5 / Opus 5.5: Free&Plus ❌, **Google AI Pro ✅\*\* «только для non-trial подписок»**, Ultra ✅, Enterprise ❌.

## Вывод

Это серверный entitlement Google, а не дефект AG Tools/v4.9.4. Каталог 5.5 не приходит от самого Google для этих аккаунтов. Причина — одна из: (a) подписка Pro в trial/промо; (b) rollout ещё не дошёл; (c) региональный/IP-гейт (в `ineligibleTiers` присутствует `UNSUPPORTED_LOCATION`, egress идёт напрямую с NL-хоста, HTTP proxy_pool 7/7 мёртв). Различить (a)/(b)/(c) по ответам API нельзя; различает только проверка подписки владельцем в Google One и/или тест аккаунта из другого egress.

Shadow после проверки остановлен; production не менялся.

## Дополнение 2026-10-06 — официальный ответ Google и канал @cryptoperchiki

Источник: Google AI Developers Forum, ответ по теме «claude Opus/Sonnet 5.5 not showing on Google AI Pro offered through jio» (discuss.ai.google.dev/t/…/186704).

- Claude 5.5 доступен на Pro только для **non-trial, напрямую оплаченных через Google One** подписок.
- Промо- и carrier-bundled планы («Offered through Jio», 18-месячные редимнутые) классифицируются как promotional/trial tier и **Claude 5.5 не получают** — это ожидаемое поведение, а не баг.
- Ключевой признак — канал оплаты подписки, а не страна аккаунта. Страна влияет только на общий допуск к Antigravity (`UNSUPPORTED_LOCATION` у нас есть только для free-tier; paid-tier распознаётся).
- Антигравити удаляет Claude 4.6 и GPT-OSS-120b **2026-11-02**. После этой даты на текущих аккаунтах Claude пропадёт полностью.

Пост t.me/cryptoperchiki/408 описывает обход: на аккаунте-организаторе создать семейную группу, добавить второй аккаунт без подписки, включить «Семейный доступ к Google One», войти в Antigravity вторым аккаунтом. Владелец подтвердил: все три наших аккаунта — организаторы без участников, куплены по той же схеме. Это совпадает с описанным в посте состоянием «модели не отображаются».

Статус: гипотеза, не доказана. Проверка — добавить участника в shadow и сравнить сырые `loadCodeAssist`/`fetchAvailableModels`.
