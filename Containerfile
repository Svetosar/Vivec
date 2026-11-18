# node:24-alpine
FROM docker.io/library/node@sha256:d8d97e479cdc64f052f81f129362168717c59783ade8dcc1abd22e1cca8dbd7c AS admin-build
WORKDIR /admin

COPY admin/package.json admin/package-lock.json ./
RUN npm ci
COPY admin/ ./
RUN npm run build

# rust:1-alpine
FROM docker.io/library/rust@sha256:cfbeb9c8c956f048a081ae034a7c8ffbd60242f3ff76b645d3a41523f665f761 AS build
WORKDIR /src

ARG CARGO_NET_OFFLINE=false
ENV CARGO_NET_OFFLINE=${CARGO_NET_OFFLINE}

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

# alpine:3.22
FROM docker.io/library/alpine@sha256:9db2934886712d877c932e07249c75a59e6ca27adfbccc05261987a17bef2d63

ARG VERSION=0.1.0
LABEL org.opencontainers.image.title="Vivec" \
      org.opencontainers.image.description="Школьная газета с мини-CMS" \
      org.opencontainers.image.version="${VERSION}"

WORKDIR /app
COPY --from=build /src/target/release/vivec /app/vivec
COPY static ./static
COPY --from=admin-build /static/admin ./static/admin

# Непривилегированный пользователь: контейнер не должен работать от root.
RUN addgroup -g 10001 vivec \
 && adduser -D -H -u 10001 -G vivec -s /sbin/nologin vivec \
 && mkdir -p /data \
 && chown vivec:vivec /data
USER 10001

ENV DATA_DIR=/data \
    RUST_LOG=info
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget -q -O /dev/null http://127.0.0.1:8080/healthz || exit 1

CMD ["/app/vivec"]
