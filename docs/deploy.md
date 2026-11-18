# Деплой Vivec

Состояние на 14.11.2026: каркас собран, образ проверен локально, **на сервер ещё
ничего не выложено**. Факты о сервере неизвестны (см. TODO «Открытые вопросы»),
поэтому шаги 2–4 здесь описаны как решение после ответов, а не как готовый скрипт.

## Правила, которые не меняются

- **Сборка только локально** (`podman build`), на сервер едет готовый образ.
  На сервере нет тулчейна и не должно быть.
- Наружу контейнер не смотрит: `-p 127.0.0.1:8081:8080`, снаружи только Caddy.
- `:Z` в томе только если на сервере SELinux. Без SELinux `:Z` безвреден, но
  лучше не тащить вслепую — проверить `getenforce`.
- Порты и домен свободны — сначала аудит, потом запуск. Панель 3x-ui и Caddy
  часто уже висят на 80/443 и на нестандартном порту панели.

## 1. Аудит сервера (выполнить на сервере, вывод прислать)

```sh
cat /etc/os-release             # ОС и версия
ps -p 1 -o comm=                # init: systemd / openrc
ss -tlnp                        # кто какие порты занял
caddy version; systemctl status caddy --no-pager 2>/dev/null || rc-service caddy status
podman --version; podman info --format '{{.Host.Security.Rootless}}'
getenforce 2>/dev/null || echo "SELinux: нет"
df -h /; free -m
dig +short NS example.ru        # есть ли DNS-зона
dig +short gazeta.example.ru    # куда указывает A-запись
```

Что из этого на что влияет:

| Факт | Зачем |
|---|---|
| init | systemd → Quadlet-юнит в `~/.config/containers/systemd/`; OpenRC → `rc-service` + `supervise-daemon` |
| `ss -tlnp` | свободный порт на loopback и кто держит 80/443 |
| `caddy version` | Caddy уже стоит или ставим с нуля |
| SELinux | `:Z` у тома с базой |
| DNS | без A-записи Caddy не выпустит сертификат |

## 2. Публикация порта и Caddy

```sh
# на сервере
# --userns=keep-id обязателен: в образе USER 10001, и без него rootless podman
# не даст процессу писать в /data (G-021).
mkdir -p /srv/vivec/data
podman run -d --name vivec \
  --userns=keep-id:uid=10001,gid=10001 \
  -p 127.0.0.1:8081:8080 \
  -v /srv/vivec/data:/data:Z \
  --restart=unless-stopped \
  vivec:latest
```

Проверка после запуска: `podman logs vivec` → `база готова: /data/vivec.db`,
`curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8081/` → `200`.
Если в логах `unable to open database file` — забыт `--userns=keep-id` (G-021).

Блок Caddy — `deploy/Caddyfile.snippet`, добавляется к существующему файлу.
Порядок: `caddy validate` → `systemctl reload caddy` → `curl -I https://домен/`.

## 3. Передача образа

```sh
# на рабочей машине
podman build -t vivec:$(date +%Y%m%d-%H%M) -t vivec:latest .
podman save vivec:latest | gzip > /tmp/vivec.tar.gz
scp /tmp/vivec.tar.gz сервер:/tmp/

# на сервере
gunzip -c /tmp/vivec.tar.gz | podman load
```

Тег с датой нужен для отката: `podman tag vivec:20261115-1200 vivec:latest` и
перезапуск. Старые теги чистить вручную, не «все сразу».

## 4. Проверка после деплоя

```sh
curl -sS -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8081/healthz   # ok
curl -sS -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8081/          # 200
curl -sS -o /dev/null -w '%{http_code}\n' https://домен/                  # 200
podman logs --tail 20 vivec
```

Остановка должна занимать меньше секунды (SIGTERM обрабатывается), иначе смотреть
`podman stop -t 5 vivec` и журнал.

## 5. Backup и восстановление

Перед обновлением и ежедневно:

```sh
VIVEC_DATA_DIR=/srv/vivec/data \
VIVEC_BACKUP_DIR=/srv/vivec/backups \
./scripts/backup.sh
```

Скрипт делает SQLite `.backup`, проверяет `PRAGMA integrity_check`, архивирует
`uploads` и пишет manifest с SHA-256. Хранение по умолчанию — 14 дней.
После создания копию и manifest нужно скопировать за пределы сервера (правило 3-2-1).

Проверка восстановления:

```sh
VIVEC_BACKUP_DIR=/srv/vivec/backups ./scripts/verify-backup.sh
```

Скрипт разворачивает последний backup во временный `DATA_DIR`, запускает приложение
и проверяет `/healthz` и главную. Расписание cron/timer добавить после аудита init-системы.

Локальная нагрузочная проверка:

```sh
./scripts/load.sh
```

По умолчанию: 200 запросов, concurrency 10, временная БД. Скрипт печатает
время, req/s и фактический RSS (`/proc/<pid>/status`, не `podman stats`).

## Ещё не сделано

- `scripts/deploy.sh` — после ответов на аудит: точные команды и проверки.
- Юнит для init-системы сервера (Quadlet или OpenRC) — зависит от `ps -p 1`.
