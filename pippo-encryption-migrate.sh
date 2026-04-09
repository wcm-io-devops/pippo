#!/usr/bin/env bash

set -euo pipefail

FILE="${1:-}"

if [[ -z "$FILE" ]]; then
  echo "Usage: $0 <yaml-file>"
  exit 1
fi

if [[ ! -f "$FILE" ]]; then
  echo "❌ File not found: $FILE"
  exit 1
fi

if [[ -z "${PIPPO_CRYPTKEY:-}" ]]; then
  echo "❌ PIPPO_CRYPTKEY not set"
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "❌ python3 is required but was not found in PATH"
  exit 1
fi

if [[ -x "./target/debug/pippo" ]]; then
  PIPPO_BIN="./target/debug/pippo"
elif command -v pippo >/dev/null 2>&1; then
  PIPPO_BIN="$(command -v pippo)"
else
  echo "❌ pippo not found. Build ./target/debug/pippo or install pippo in PATH"
  exit 1
fi

cp "$FILE" "$FILE.bak"

while IFS= read -r old; do
  echo "Processing: $old"
  plaintext=$("$PIPPO_BIN" decrypt "$old")
  new=$("$PIPPO_BIN" encrypt "$plaintext")

  python3 - "$FILE" "$old" "$new" <<'PY'
import sys
from pathlib import Path

file_path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
p = Path(file_path)
text = p.read_text()
text = text.replace(old, new)
p.write_text(text)
PY

done < <(grep -o '\$enc [^"[:space:]]*' "$FILE")

echo "✅ Migration complete: $FILE"
echo "📦 Backup created at: $FILE.bak"