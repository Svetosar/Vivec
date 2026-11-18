#!/usr/bin/env bash
set -euo pipefail

requests="${LOAD_REQUESTS:-200}"
concurrency="${LOAD_CONCURRENCY:-10}"
port="${SMOKE_PORT:-18083}"
tmp_dir="$(mktemp -d /tmp/vivec-load.XXXXXX)"
data_dir="$tmp_dir/data"
status_file="$tmp_dir/statuses"
log_file="$tmp_dir/vivec.log"
url="http://127.0.0.1:$port/"
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
    [ -f "$log_file" ] && tail -20 "$log_file" >&2
    exit 1
}

cd "$(dirname "$0")/.."
cargo build --quiet --bin vivec

DATA_DIR="$data_dir" PORT="$port" ./target/debug/vivec >"$log_file" 2>&1 &
app_pid=$!

for _ in $(seq 1 50); do
    if curl -fsS "${url}healthz" >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done
curl -fsS "${url}healthz" >/dev/null || fail "сервер не запустился"

sqlite3 "$data_dir/vivec.db" <migrations/seed.sql
for _ in $(seq 1 10); do
    curl -fsS -o /dev/null "$url"
done

export LOAD_URL="$url"
started="$(date +%s%N)"
seq 1 "$requests" |
    xargs -P "$concurrency" -n 1 sh -c \
        'curl -sS -o /dev/null -w "%{http_code}\n" "$LOAD_URL"' >"$status_file"
finished="$(date +%s%N)"

bad="$(awk '$1 != 200 { bad += 1 } END { print bad + 0 }' "$status_file")"
[ "$bad" -eq 0 ] || fail "неуспешных ответов: $bad"

elapsed_ms="$(( (finished - started) / 1000000 ))"
rps="$(awk -v n="$requests" -v ms="$elapsed_ms" \
    'BEGIN { if (ms > 0) printf "%.1f", n * 1000 / ms; else print "inf" }')"
rss="$(awk '/VmRSS:/ { print $2 " " $3 }' "/proc/$app_pid/status")"

printf 'LOAD PASS: %s запросов, concurrency %s, %s ms, %s req/s, RSS %s\n' \
    "$requests" "$concurrency" "$elapsed_ms" "$rps" "$rss"
