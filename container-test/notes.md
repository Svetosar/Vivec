# Тестовый контейнер Vivec

Только локальный тест. Порт опубликован на `127.0.0.1`, наружу контейнер
не смотрит.

## Запуск

```sh
cd container-test
podman-compose up -d --build
```

Открыть:

```text
Публичная газета: http://127.0.0.1:8081/
Админка:          http://127.0.0.1:8081/admin/
```

Тестовый вход:

```text
Пользователь: editor
Пароль:       vivec-test
```

При первом запуске в пустую базу автоматически добавляются четыре
демонстрационные статьи.

## Полезные команды

```sh
podman-compose ps
podman-compose logs -f vivec
podman-compose restart vivec
podman-compose down
```

Обычный `down` сохраняет named volume с базой. Полный сброс тестовых данных:

```sh
podman-compose down -v
```

Повторная сборка после изменения Rust или Vue:

```sh
podman-compose up -d --build
```

## Ограничения тестовой конфигурации

- пароль известный и предназначен только для локального теста;
- `SESSION_COOKIE_SECURE=false` нужен для HTTP на localhost;
- Caddy, TLS и SELinux bind mount здесь не проверяются;
- production-конфигурация должна брать секреты из `deploy/vivec.env`.
