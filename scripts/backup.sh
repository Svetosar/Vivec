#!/usr/bin/env bash
set -euo pipefail

data_dir="${VIVEC_DATA_DIR:-/srv/vivec/data}"
backup_dir="${VIVEC_BACKUP_DIR:-/srv/vivec/backups}"
retention_days="${VIVEC_BACKUP_RETENTION_DAYS:-14}"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
source_db="$data_dir/vivec.db"
target_db="$backup_dir/vivec-$timestamp.db"
target_uploads="$backup_dir/uploads-$timestamp.tar.gz"
manifest="$backup_dir/manifest-$timestamp.txt"

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

[ -f "$source_db" ] || fail "база не найдена: $source_db"
command -v sqlite3 >/dev/null || fail "sqlite3 не установлен"
mkdir -p "$backup_dir"

sqlite3 "$source_db" ".backup '$target_db'"
integrity="$(sqlite3 "$target_db" 'PRAGMA integrity_check;')"
[ "$integrity" = "ok" ] || fail "integrity_check: $integrity"

uploads_count=0
if [ -d "$data_dir/uploads" ]; then
    tar -C "$data_dir" -czf "$target_uploads" uploads
    uploads_count="$(find "$data_dir/uploads" -type f | wc -l | tr -d ' ')"
fi

article_count="$(sqlite3 "$target_db" 'SELECT COUNT(*) FROM articles;')"
db_size="$(du -h "$target_db" | cut -f1)"
{
    printf 'created_at=%s\n' "$timestamp"
    printf 'source_db=%s\n' "$source_db"
    printf 'database=%s\n' "$(basename "$target_db")"
    printf 'database_sha256=%s\n' "$(sha256sum "$target_db" | cut -d' ' -f1)"
    printf 'articles=%s\n' "$article_count"
    printf 'uploads_files=%s\n' "$uploads_count"
    if [ -f "$target_uploads" ]; then
        printf 'uploads=%s\n' "$(basename "$target_uploads")"
        printf 'uploads_sha256=%s\n' "$(sha256sum "$target_uploads" | cut -d' ' -f1)"
    fi
} >"$manifest"

find "$backup_dir" -maxdepth 1 -type f -name 'vivec-*.db' \
    -mtime +"$retention_days" -delete
find "$backup_dir" -maxdepth 1 -type f -name 'uploads-*.tar.gz' \
    -mtime +"$retention_days" -delete
find "$backup_dir" -maxdepth 1 -type f -name 'manifest-*.txt' \
    -mtime +"$retention_days" -delete

printf 'BACKUP PASS: %s (%s, статей: %s, uploads: %s)\n' \
    "$target_db" "$db_size" "$article_count" "$uploads_count"
printf 'Скопируйте manifest и backup за пределы сервера: правило 3-2-1.\n'
