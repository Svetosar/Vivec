# node:24-slim
FROM docker.io/library/node@sha256:0e2646ab57e77b7a384a0cb59b6815b182c77e1f9c3ba36bd8435bd9c4e6680e AS admin-build
WORKDIR /admin

COPY admin/package.json admin/package-lock.json ./
RUN npm ci
COPY admin/ ./
RUN npm run build

# rust:1-slim-bookworm
FROM docker.io/library/rust@sha256:476f6598b8d14a1f1bc7e58efc67af206ac8a559c37a3401ce512db2582d0ff7 AS build
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
# Собираем пустой main, чтобы закэшировать слой с зависимостями:
# правка своего кода не будет пересобирать все крейты заново.
RUN mkdir -p src \
 && echo 'fn main() {}' > src/main.rs \
 && cargo build --release \
 && rm -rf src

COPY src ./src
COPY templates ./templates
COPY migrations ./migrations
COPY static ./static

# ВАЖНО: COPY сохраняет mtime исходников с хоста. Если они старше
# артефакта от пустой сборки, cargo решит, что менять нечего, и в образ
# попадёт пустой бинарник (exit 0, пустые логи). Поэтому трогаем файлы.
RUN find src templates migrations -type f -exec touch {} + \
 && cargo build --release

# debian:bookworm-slim
FROM docker.io/library/debian@sha256:c67e92c8099ba22853ab4cfda7cfb494d253d6012ca82dee98ba8976772af018
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /src/target/release/vivec /app/vivec
COPY static ./static
COPY --from=admin-build /static/admin ./static/admin

# Непривилегированный пользователь: контейнер не должен работать от root.
RUN useradd --uid 10001 --user-group --no-create-home --shell /usr/sbin/nologin vivec \
 && mkdir -p /data \
 && chown vivec:vivec /data
USER 10001

ENV DATA_DIR=/data \
    RUST_LOG=info
EXPOSE 8080

CMD ["/app/vivec"]
