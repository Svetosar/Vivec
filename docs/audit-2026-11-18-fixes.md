# Исправления аудита 18.11.2026

## P1

- Логин ограничен по username и по IP; map ограничены 1024 записями и очищаются.
- Неизвестный username отсекается до Argon2.
- Argon2 выполняется через `spawn_blocking` под семафором на одну проверку.
- `X-Forwarded-For` берётся как последний адрес, добавленный Caddy; доверие
  заголовку включается только через `TRUST_PROXY=true`.
- Stress-проверка: 1100 уникальных username/IP → 1024 × 401, 76 × 429,
  RSS 17,6 → 20,0 МБ.

## P2

- `SESSION_COOKIE_SECURE` по умолчанию `true`; локальный тест явно ставит `false`.
- Добавлены CSP, `frame-ancestors 'none'`, `X-Frame-Options: DENY`, HSTS в Caddy.
- `/api/admin/*` получает `Cache-Control: no-store`.
- Главная, рубрики, поиск и архив используют `ArticleSummary` без Markdown.
- Дата считается на каждый запрос в московском времени `+03:00`; публикация
  записывает московскую дату.
- Добавлены `PUBLIC_ORIGIN`, description, canonical, Open Graph, Twitter card.
- Добавлены `/archive`, `/robots.txt`, `/sitemap.xml`, favicon и noindex поиска.
- 401 в `api.js` централизованно уводит на login с `next`.
- Редактор защищает несохранённый текст через dirty guard, `beforeunload` и Ctrl+S.
- Добавлены `deploy/vivec.env.example` и `deploy/vivec.container`.

## P3

- Зарезервированные slug запрещены.
- Добавлены `Cache-Control` для admin assets, uploads, CSS, HTML и SEO routes.
- Старые upload-файлы удаляются после update/delete, если на них больше нет ссылок.
- Добавлен общий лимит uploads 256 МиБ.
- Markdown-картинки получают `loading="lazy"` и `decoding="async"`.
- Поиск получил заметный `:focus-visible`.
- Admin index: `lang="ru"` и noindex.
- Тестовый контейнер: read-only root, tmpfs, cap-drop, no-new-privileges.

## Проверено

- `cargo test` — 11 тестов.
- `cargo clippy --all-targets -- -D warnings`.
- `npm run lint`, `npm run build`.
- `scripts/smoke.sh` — расширенный набор, включая SEO, cache headers, rate limit,
  reserved slug, lazy image и upload cleanup.
- Headless Chrome: dirty guard и 401 redirect.
- Hardened image rebuilt; login, headers, archive, robots, sitemap и upload работают.

## Осталось

- Админский bundle всё ещё занимает около 2,3 МБ и 126 файлов. Проверено, что
  language chunks не входят в первичный preload; editor грузит их только на своём
  route. Оптимизацию можно отложить до feature freeze.
- Образы `node:24-slim`, `rust:1-slim-bookworm`, `debian:bookworm-slim` не
  закреплены digest. Сделать перед release, когда будет выбран финальный toolchain.
- `docs/backend-guide.md` написан до аудита и требует обновления после стабилизации.
- Ветки не слиты в `main`; нужен финальный чек-лист и явное разрешение.
