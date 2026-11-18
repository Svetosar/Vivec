#!/usr/bin/env bash
set -euo pipefail

backup_db="${1:-}"
backup_uploads="${2:-}"
backup_dir="${VIVEC_BACKUP_DIR:-/srv/vivec/backups}"
port="${SMOKE_PORT:-18082}"
tmp_dir="$(mktemp -d /tmp/vivec-restore.XXXXXX)"
data_dir="$tmp_dir/data"
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

cd "$(dirname "$0")/.."

if [ -z "$backup_db" ]; then
    backup_db="$(find "$backup_dir" -maxdepth 1 -type f -name 'vivec-*.db' \
        -printf '%T@ %p\n' | sort -nr | head -1 | cut -d' ' -f2-)"
fi
[ -n "$backup_db" ] && [ -f "$backup_db" ] || fail "backup БД не найден"

if [ -z "$backup_uploads" ]; then
    timestamp="${backup_db##*/vivec-}"
    timestamp="${timestamp%.db}"
    candidate="$backup_dir/uploads-$timestamp.tar.gz"
    if [ -f "$candidate" ]; then
        backup_uploads="$candidate"
    fi
fi

integrity="$(sqlite3 "$backup_db" 'PRAGMA integrity_check;')"
[ "$integrity" = "ok" ] || fail "integrity_check: $integrity"
articles="$(sqlite3 "$backup_db" 'SELECT COUNT(*) FROM articles;')"

mkdir -p "$data_dir"
cp "$backup_db" "$data_dir/vivec.db"
if [ -n "$backup_uploads" ] && [ -f "$backup_uploads" ]; then
    tar -C "$data_dir" -xzf "$backup_uploads"
fi

cargo build --quiet --bin vivec
DATA_DIR="$data_dir" \
PORT="$port" \
./target/debug/vivec >"$log_file" 2>&1 &
app_pid=$!

for _ in $(seq 1 50); do
    if curl -fsS "http://127.0.0.1:$port/healthz" >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

curl -fsS "http://127.0.0.1:$port/healthz" >/dev/null || fail "restored server не запустился"
curl -fsS -o /dev/null "http://127.0.0.1:$port/" || fail "главная не открылась"
sessions="$(sqlite3 "$data_dir/vivec.db" 'SELECT COUNT(*) FROM sessions;')"

printf 'RESTORE PASS: статей=%s, сессий=%s, uploads=%s\n' \
    "$articles" "$sessions" "${backup_uploads:-нет}"
