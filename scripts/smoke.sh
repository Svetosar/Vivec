#!/usr/bin/env bash
set -euo pipefail

port="${SMOKE_PORT:-18081}"
tmp_dir="$(mktemp -d /tmp/vivec-smoke.XXXXXX)"
data_dir="$tmp_dir/data"
cookie_file="$tmp_dir/cookies.txt"
body_file="$tmp_dir/body"
log_file="$tmp_dir/vivec.log"
app_pid=""

cleanup() {
    if [ -n "$app_pid" ]; then
        kill "$app_pid" 2>/dev/null || true
        wait "$app_pid" 2>/dev/null || true
    fi
    rm -rf "$tmp_dir"
}
trap cleanup EXIT

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    if [ -f "$log_file" ]; then
        tail -20 "$log_file" >&2
    fi
    exit 1
}

expect_code() {
    expected="$1"
    title="$2"
    shift 2
    actual="$(curl -sS -o "$body_file" -w '%{http_code}' "$@")"
    if [ "$actual" != "$expected" ]; then
        fail "$title: ожидался HTTP $expected, получен $actual"
    fi
    printf 'ok: %s\n' "$title"
}

cd "$(dirname "$0")/.."

if [ ! -f static/admin/index.html ]; then
    fail "static/admin/index.html отсутствует — выполните npm --prefix admin run build"
fi

cargo build --quiet --bin vivec
password_hash="$(printf 'vivec-smoke-password\n' | cargo run --quiet --bin hash-password)"

DATA_DIR="$data_dir" \
PORT="$port" \
ADMIN_USER=editor \
ADMIN_PASSWORD_HASH="$password_hash" \
SESSION_COOKIE_SECURE=false \
./target/debug/vivec >"$log_file" 2>&1 &
app_pid=$!

for _ in $(seq 1 50); do
    if curl -fsS "http://127.0.0.1:$port/healthz" >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done
curl -fsS "http://127.0.0.1:$port/healthz" >/dev/null || fail "сервер не запустился"
printf 'ok: сервер запущен\n'

expect_code 200 "главная" "http://127.0.0.1:$port/"
expect_code 200 "CSS" "http://127.0.0.1:$port/static/style.css"
expect_code 200 "SPA админки" "http://127.0.0.1:$port/admin/articles"
admin_asset="$(sed -n 's/.*src="\([^"]*\.js\)".*/\1/p' "$body_file" | head -1)"
[ -n "$admin_asset" ] || fail "admin asset не найден"
curl -sS -D "$tmp_dir/admin-asset.headers" -o /dev/null \
    "http://127.0.0.1:$port$admin_asset"
grep -qi '^cache-control: public, max-age=31536000, immutable' \
    "$tmp_dir/admin-asset.headers" || fail "admin asset cache-control"
expect_code 401 "API без cookie" "http://127.0.0.1:$port/api/admin/me"
expect_code 403 "login без CSRF" \
    -H 'Content-Type: application/json' \
    -d '{"username":"editor","password":"vivec-smoke-password"}' \
    "http://127.0.0.1:$port/api/admin/login"

expect_code 200 "login" \
    -c "$cookie_file" \
    -D "$tmp_dir/login.headers" \
    -H 'X-Vivec-CSRF: 1' \
    -H 'Content-Type: application/json' \
    -d '{"username":"editor","password":"vivec-smoke-password"}' \
    "http://127.0.0.1:$port/api/admin/login"
grep -qi '^set-cookie: vivec_session=' "$tmp_dir/login.headers" || fail "cookie не установлена"

curl -sS -D "$tmp_dir/api.headers" -o /dev/null \
    -b "$cookie_file" "http://127.0.0.1:$port/api/admin/me"
grep -qi '^cache-control: no-store' "$tmp_dir/api.headers" || fail "API cache-control"
curl -sS -D "$tmp_dir/html.headers" -o /dev/null "http://127.0.0.1:$port/"
grep -qi '^content-security-policy:' "$tmp_dir/html.headers" || fail "CSP отсутствует"
grep -qi '^x-frame-options: DENY' "$tmp_dir/html.headers" || fail "X-Frame-Options"
grep -qi '^cache-control: no-cache' "$tmp_dir/html.headers" || fail "HTML cache-control"
curl -sS -D "$tmp_dir/css.headers" -o /dev/null "http://127.0.0.1:$port/static/style.css"
grep -qi '^cache-control: public, max-age=3600' "$tmp_dir/css.headers" || fail "CSS cache-control"

expect_code 201 "создание черновика" \
    -b "$cookie_file" \
    -H 'X-Vivec-CSRF: 1' \
    -H 'Content-Type: application/json' \
    -d '{"title":"Смоук-проверка","slug":"smoke-check","rubric":"Школа","author":"Smoke","lead":"Проверка публикации","body_md":"Текст проверки."}' \
    "http://127.0.0.1:$port/api/admin/articles"

expect_code 400 "reserved slug" \
    -b "$cookie_file" \
    -H 'X-Vivec-CSRF: 1' \
    -H 'Content-Type: application/json' \
    -d '{"title":"Admin","slug":"admin","rubric":"Школа","author":"Smoke","lead":"","body_md":""}' \
    "http://127.0.0.1:$port/api/admin/articles"

expect_code 404 "черновик скрыт" "http://127.0.0.1:$port/smoke-check"
expect_code 200 "публикация" \
    -b "$cookie_file" \
    -H 'X-Vivec-CSRF: 1' \
    -X POST \
    "http://127.0.0.1:$port/api/admin/articles/1/publish"
expect_code 200 "опубликованная статья" "http://127.0.0.1:$port/smoke-check"
expect_code 200 "архив" "http://127.0.0.1:$port/archive"
grep -q 'Смоук-проверка' "$body_file" || fail "архив не содержит статью"
expect_code 200 "robots.txt" "http://127.0.0.1:$port/robots.txt"
grep -q 'Disallow: /admin' "$body_file" || fail "robots не закрывает admin"
curl -sS -D "$tmp_dir/sitemap.headers" -o "$tmp_dir/sitemap.xml" \
    "http://127.0.0.1:$port/sitemap.xml"
grep -qi '^content-type: application/xml' "$tmp_dir/sitemap.headers" || fail "sitemap content-type"
grep -q '/smoke-check' "$tmp_dir/sitemap.xml" || fail "sitemap не содержит статью"
curl -sS -o "$tmp_dir/home.html" "http://127.0.0.1:$port/"
grep -q 'property="og:title"' "$tmp_dir/home.html" || fail "OG title отсутствует"
grep -q 'rel="canonical"' "$tmp_dir/home.html" || fail "canonical отсутствует"

expect_code 200 "поиск кириллицы" \
    -G --data-urlencode 'q=СМОУК' \
    "http://127.0.0.1:$port/search"
grep -q 'Смоук-проверка' "$body_file" || fail "поиск не нашёл статью"

expect_code 200 "XSS-текст" \
    -b "$cookie_file" \
    -H 'X-Vivec-CSRF: 1' \
    -H 'Content-Type: application/json' \
    -X PUT \
    -d '{"title":"Смоук-проверка","slug":"smoke-check","rubric":"Школа","author":"Smoke","lead":"Проверка публикации","body_md":"<script>alert(1)</script>"}' \
    "http://127.0.0.1:$port/api/admin/articles/1"
expect_code 200 "статья после XSS" "http://127.0.0.1:$port/smoke-check"
if grep -q '<script>' "$body_file"; then
    fail "ammonia пропустила script"
fi
printf 'ok: script удалён\n'

expect_code 200 "upload PNG" \
    -b "$cookie_file" \
    -H 'X-Vivec-CSRF: 1' \
    -F 'file=@design/preview/home-mobile.png' \
    "http://127.0.0.1:$port/api/admin/uploads"
upload_url="$(sed -n 's/.*"url":"\([^"]*\)".*/\1/p' "$body_file")"
[ -n "$upload_url" ] || fail "upload не вернул URL"
expect_code 200 "раздача upload" "http://127.0.0.1:$port$upload_url"

expect_code 200 "снятие с публикации" \
    -b "$cookie_file" \
    -H 'X-Vivec-CSRF: 1' \
    -X POST \
    "http://127.0.0.1:$port/api/admin/articles/1/unpublish"
expect_code 404 "снятая статья скрыта" "http://127.0.0.1:$port/smoke-check"
expect_code 204 "удаление" \
    -b "$cookie_file" \
    -H 'X-Vivec-CSRF: 1' \
    -X DELETE \
    "http://127.0.0.1:$port/api/admin/articles/1"

image_payload="$(printf '{"title":"Upload smoke","slug":"upload-smoke","rubric":"Школа","author":"Smoke","lead":"","body_md":"![x](%s)"}' "$upload_url")"
expect_code 201 "статья с upload" \
    -b "$cookie_file" \
    -H 'X-Vivec-CSRF: 1' \
    -H 'Content-Type: application/json' \
    -d "$image_payload" \
    "http://127.0.0.1:$port/api/admin/articles"
expect_code 200 "публикация upload-статьи" \
    -b "$cookie_file" \
    -H 'X-Vivec-CSRF: 1' \
    -X POST \
    "http://127.0.0.1:$port/api/admin/articles/2/publish"
expect_code 200 "lazy image" "http://127.0.0.1:$port/upload-smoke"
grep -q 'loading="lazy"' "$body_file" || fail "изображение не lazy"
expect_code 204 "удаление статьи с upload" \
    -b "$cookie_file" \
    -H 'X-Vivec-CSRF: 1' \
    -X DELETE \
    "http://127.0.0.1:$port/api/admin/articles/2"
expect_code 404 "осиротевший upload удалён" "http://127.0.0.1:$port$upload_url"

for index in $(seq 1 20); do
    curl -sS -o /dev/null \
        -H 'X-Vivec-CSRF: 1' \
        -H 'X-Forwarded-For: 203.0.113.77' \
        -H 'Content-Type: application/json' \
        -d "{\"username\":\"unknown-$index\",\"password\":\"bad\"}" \
        "http://127.0.0.1:$port/api/admin/login"
done
expect_code 429 "IP rate limit" \
    -H 'X-Vivec-CSRF: 1' \
    -H 'X-Forwarded-For: 203.0.113.77' \
    -H 'Content-Type: application/json' \
    -d '{"username":"unknown-21","password":"bad"}' \
    "http://127.0.0.1:$port/api/admin/login"

printf 'SMOKE PASS\n'
