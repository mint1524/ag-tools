# AG Tools: обновление и диагностика 503
Дата: 2026-11-13. Тип: краткая ops-spec (<1 дня), implementation approved.
Владелец запросил обновить ai.okak.club и выяснить 503; отдельно выбрал: «Разрешаю тебе обновить production после проверки, с откатом при проблеме». Это разовый override прежнего owner-only Deploy, не разрешение иных релизов.

## Вход и границы
Живой Swarm sidequests-agtools-a1xtnr, volume ag_tools_data, v4.5.1. Health 200; 1 индексированный Google-аккаунт. Два разных model probes дают 429; точный body содержит upstream HTTP403 TOS_VIOLATION / PERMISSION_DENIED и просьбу подать апелляцию. Старые 3-account/Claude5.5 требования не являются приёмкой этой maintenance-задачи.

## Требования
### FR-1 — причина (Must)
WHEN клиент получает 503/исчерпание пула, система диагностики SHALL отличить site health, auth, quota и upstream ban по реальному запросу и логам.
- AC-1.1 Given живой production, When выполнены health/catalog/два model probes, Then записаны статусы и upstream причина без секретов.
### FR-2 — сохранность (Must)
WHEN готовится обновление, оператор SHALL создать приватный restorable backup конфигурации, индексированных credentials и SQLite; проверить JSON/DB integrity на восстановленной копии.
- AC-2.1 Given backup, When архив восстановлен, Then JSON валидны, credentials/config сохранены, SQLite integrity_check=ok; checksum/mode записаны.
### FR-3 — кандидат (Must)
WHEN выбран стабильный upstream 5.0.3, оператор SHALL закрепить digest и испытать его на отдельной копии с loopback-only ingress дважды.
- AC-3.1 Given shadow, When выполнены два старта, Then health version=5.0.3, account identities и client credentials не теряются, DB integrity=ok, auth закрыт.
- AC-3.2 Given заблокированный аккаунт, When выполнен bounded generation probe, Then реальный отказ честно отмечен; успех генерации не имитируется и блокировку не обходят.
### FR-4 — production (Must)
WHEN backup/shadow прошли compatibility gates, оператор SHALL обновить только ag-tools через Dokploy и проверить его сохранённый pinned image и фактический Swarm runtime.
- AC-4.1 Given owner override, When Deploy завершён, Then health200/version5.0.3, UI assets200, catalog с auth, без auth отказ, прежние client key hashes и индексированный аккаунт сохранены.
- AC-4.2 Given инфраструктурная/миграционная регрессия, When проверка провалилась, Then восстановлены старые image и при необходимости данные. Известный Google ban не является причиной откатывать исправный бинарь.
### FR-5 — восстановление генерации (Must, внешний gate)
WHEN Google снимет ban либо владелец добавит разрешённый аккаунт, сервис SHALL пройти Gemini и Claude generation smoke. До этого результат SHALL называться BLOCKED_EXTERNAL, а не «всё работает».
- AC-5.1 Given текущий ban, When работа завершена, Then показан точный owner unblock и генерация остаётся явно непроверенной/заблокированной.
### FR-6 — прямое соединение (Must, owner-approved amendment)
WHEN мёртвый привязанный пул препятствует upstream, оператор SHALL отключить только proxy.proxy_pool.enabled, сохранив список и привязки для отката.
- AC-6.1 Given отдельное разрешение владельца 2026-11-13 «Разрешаю отключить мёртвый пул прокси и подключаться к Google напрямую», When shadow и production проверены, Then pool.enabled=false; Google видит IP сервера, без обхода ToS ban. Это единственное разрешённое изменение egress.
### NFR-1 — безопасность
Оператор SHALL не раскрывать ключи/account emails/tokens, не менять Hermes profiles, другие сервисы, DNS, тарифы, маршрутизацию на иной LLM; не подавать апелляцию от имени владельца.
### NFR-2 — bounded operations
Оператор SHALL использовать готовые images (без host build), ограниченный shadow, не удалять старые backup/volumes и не отключать guards ради зелёного smoke.
