# TODO — Vivec

Обновлено: 18.11.2026
Правило: перед задачей — прочитать, после — отметить. Список не живёт в голове.

## Сейчас

- [x] Зафиксировать план, стек и правила (скилл `vivec`, `AGENTS.md`)
- [x] 14.11 — каркас проекта + контейнер (сделано 13.11)
- [x] 14.11 — FigCraft MCP подключён и проверен (скриншот + экспорт CSS)
- [x] 14.11 — дизайн: 3 артборда в `design/vivec.figcraft` + вёрстка `static/style.css`
- [ ] 15.11 — сервер: **нужен аудит** (`docs/deploy.md`), затем `scripts/deploy.sh` и юнит
- [x] 16.11 — схема, пул, сид, список, статья, 404 (сделано 14.11); G-021 закрыт 15.11
- [x] 17.11 — markdown + ammonia, рубрики, транслит slug (сделано 15.11) — **публичная часть готова**
- [x] 17.11 — подробный учебный гайд `docs/guide.md`
- [x] 17.11 — правило provenance: каждый коммит помечается `AI-assisted: Codex`
- [x] 17.11 — README/LICENSE не редактируются ИИ, но включаются в коммиты как есть
- [x] 18.11 — визуальный smoke login/list/editor на desktop и 390 px
- [x] 18.11 — подробный построчный гайд по backend `docs/backend-guide.md`
- [x] 18.11 — тестовый multi-stage image и `container-test` через podman-compose
- [x] 18.11 — аудит безопасности, оптимизации, SEO и UX → `docs/audit-2026-11-18.md`
- [x] 18.11 — P1/P2 и основные P3 аудита исправлены → `docs/audit-2026-11-18-fixes.md`

## План по дням

### Фаза A. Фундамент
- [x] **14.11** — `Cargo.toml`, `main.rs` + `/healthz`, `error.rs`, `Containerfile`, `.dockerignore`, `podman run` локально
- [x] **14.11** — дизайн: 3 артборда (`home-desktop`, `article-desktop`, `home-mobile`), CSS и шаблоны
- [ ] **15.11** — сервер: аудит (порты, DNS, Caddy, SELinux, init), site-блок Caddy, `scripts/deploy.sh`, контейнер на проде *(ждёт аудит от пользователя; G-021 в контейнере уже закрыт)*
- [x] **16.11** — `0001_init.sql`, пул, сид 4 статей, список, страница статьи, 404 *(сделано 14.11)*
- [x] **17.11** — markdown + ammonia, рубрики, транслит slug, CSS → **публичная часть готова** *(сделано 15.11)*

### Фаза B. Админка
- [x] **18.11** — сессии, argon2, cookie, middleware `/admin/*`, `/api/admin/me` *(сделано 17.11)*
- [x] **19.11** — JSON API: list/get/create/update/publish/delete, `AppError` *(сделано 17.11)*
- [x] **20.11** — `create-vue`, router, логин, таблица статей *(сделано 17.11)*
- [x] **21.11** — форма статьи + `md-editor-v3` *(сделано 17.11, визуальный smoke 18.11)*
- [x] **22.11** — загрузка картинок (multipart, `infer`, лимит 8 МБ) *(сделано 17.11)*

### Фаза C. Доводка
- [x] **23.11** — поиск, навигация, мобильная вёрстка *(сделано 18.11; Unicode-поиск вместо `LIKE`)*
- [x] **24.11** — безопасность: CSRF-заголовок, лимит логина, заголовки, лимиты тела *(сделано 17.11)*
- [x] **25.11** — `smoke.sh`, чек-лист на 20 пунктов, фикс багов *(сделано 18.11)*
- [ ] **26.11** — **FEATURE FREEZE**, README, инструкция редактору

### Фаза D. Сдача
- [ ] **27.11** — репетиция деплоя с нуля по README
- [x] **28.11** — бэкапы + **проверенное восстановление** + нагрузка *(сделано 18.11)*
- [ ] **29.11** — только баги, 5–7 реальных статей, скриншоты
- [ ] **30.11** — демо на 5 минут, тег `v1.0`

## В конце каждой сессии

- [ ] Запись в `docs/journal.md`: сделано → новое → сломалось → урок → дальше
- [ ] Новые ловушки — в `docs/gotchas.md` со статусом проверки
- [ ] Отметить сделанное здесь, незакрытое перенести
- [ ] Коммит в текущую ветку
- [ ] Воскресенье: ревизия журнала — что повторилось и пора поднять в скилл

## Открытые вопросы (спросить, не придумывать)

- [ ] ОС и init-система сервера (systemd? OpenRC?)
- [ ] Домен/поддомен для газеты, есть ли DNS-доступ
- [ ] Текущий `Caddyfile`: какие site-блоки уже есть, где слушает панель 3x-ui
- [ ] SELinux на сервере (влияет на `:Z` в томах)
- [ ] Один админ или несколько редакторов (пока один)
- [x] `src/1.html` — удалён пользователем намеренно, зафиксировано в git

## Потом (только после сдачи)

- FTS5-поиск с `trigram` вместо `LIKE`
- Импорт `.md` файлов с front-matter
- История правок статей
- RSS + sitemap
- Автосохранение черновика

## Сделано

- 13.11 — план, стек, скилл `vivec`, `AGENTS.md`, этот файл, журнал и накопитель грабель
- 13.11 — редактор выбран: VS Code, расширения установлены, `.vscode/` в репозитории
- 14.11 — FigCraft MCP: `opencode.json`, `design/`, проверено создание слоёв, скриншот и `inspect.code`
- 14.11 — дизайн: `design/vivec.figcraft` + `design/preview/*.png`, `static/style.css` по макету, шаблоны шапки/подвала/главной
- 14.11 — `docs/deploy.md` (аудит сервера + порядок деплоя), `deploy/Caddyfile.snippet`
- 14.11 — `PORT` из окружения (локально 8080 занят steamwebhelper, G-018)
- 14.11 — база: `migrations/0001_init.sql` + `seed.sql`, `src/db.rs`, `src/articles.rs`, маршрут `/{slug}`, страница 404; sqlx 0.9 и minijinja с фичей `serde`
- 15.11 — контейнер пишет в bind-mount: `--userns=keep-id:uid=10001,gid=10001` (G-021)
- 15.11 — markdown: `pulldown-cmark` + `ammonia` (`src/markdown.rs`), транслит (`src/slug.rs`, тесты), рубрики `/rubric/{slug}`, шаблон рубрики, стили markdown в статье
- 17.11 — подробный учебный гайд по Rust, SQL, auth, API, шаблонам, JS/Vue и процессу проекта
- 17.11 — auth: Argon2id, серверные сессии в SQLite, cookie, rate limit, `/api/admin/me`; проверено login/logout
- 17.11 — admin API: CRUD статей и публикация; проверено черновиком, публикацией, `409`, удалением
- 17.11 — каждый новый коммит обязан содержать строку `AI-assisted: Codex`
- 17.11 — `src/1.html` удалён пользователем намеренно; README и юридические файлы редактирует только пользователь, но они включаются в коммиты
- 17.11 — Vue 3/Vite/router/login/list/editor, `md-editor-v3`, responsive CSS; сборка проходит
- 17.11 — upload (`infer`, JPEG/PNG/WebP/GIF, 8 МиБ), `/uploads`, CSRF и security headers
- 17.11 — SPA fallback `/admin/*` исправлен и проверен; `/admin/articles` и asset отдают `200`
- 18.11 — следующий шаг: визуальный smoke login/list/editor на desktop и 390 px, затем поиск
- 18.11 — визуальный smoke пройден; мобильный список переведён в карточки, Markdown editor одноколоночный
- 18.11 — публичный поиск по title/author/lead/body, актуальная дата, Markdown-изображения, mobile nav
- 18.11 — поиск проверен в верхнем/нижнем регистре, черновики исключены, XSS-запрос экранирован
- 18.11 — `scripts/smoke.sh`: 20 проверок auth/API/поиска/upload/XSS на временной БД
- 18.11 — `scripts/backup.sh` + `scripts/verify-backup.sh`; восстановление 4 статей проверено живым запуском
- 18.11 — `scripts/load.sh`: 200 запросов, 10 параллельно, 221.7 req/s, RSS 22 МБ локально
- 18.11 — backend-гайд: Rust-синтаксис, Cargo, SQL, auth, API, upload, main, template и Containerfile
- 18.11 — образ `localhost/vivec:test`: Vue builder + Rust release + Debian runtime без Node
- 18.11 — `container-test/`: login `editor/vivec-test`, 4 демо-статьи, named volume, порт `127.0.0.1:8081`
- 18.11 — pre-auth DoS закрыт: username+IP limit, bounded map, Argon2 semaphore/spawn_blocking
- 18.11 — CSP/HSTS/no-store, Secure default, ArticleSummary, московская дата, SEO, archive, 401/dirty guard
- 18.11 — reserved slug, cache headers, orphan upload cleanup, lazy images, hardened container
- 18.11 — расширенный smoke: 30+ проверок; stress limiter: 1100 запросов, RSS +2,3 МБ

## После перерыва

- [x] Обновить `docs/backend-guide.md` под код после аудита.
- [x] Проверить bundle: language chunks не входят в первичный preload.
- [x] Закрепить digest базовых образов в `Containerfile`.
- [ ] Финальный чек-лист и слияние веток в `main` только после явного «да».
