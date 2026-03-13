#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-http://127.0.0.1:7860}"
TARGET_URL="${2:-https://komiku.org/martial-peak-chapter-980/}"
REQUESTS="${REQUESTS:-30}"
CONCURRENCY="${CONCURRENCY:-4}"

TMP_FILE="$(mktemp)"
trap 'rm -f "$TMP_FILE"' EXIT

echo "load-test base=$BASE_URL target=$TARGET_URL requests=$REQUESTS concurrency=$CONCURRENCY"

seq 1 "$REQUESTS" | xargs -P "$CONCURRENCY" -I{} sh -c '
  code_time=$(curl -sS -o /dev/null -w "%{http_code} %{time_total}" "'"$BASE_URL"'/mirror/'"$TARGET_URL"'")
  echo "$code_time"
' >> "$TMP_FILE"

echo "---- sample results (first 10) ----"
head -n 10 "$TMP_FILE" || true

echo "---- summary ----"
awk '
{
  code=$1;
  t=$2+0;
  total++;
  sum+=t;
  if (code ~ /^2/) ok++;
  if (t>max) max=t;
}
END {
  avg=(total>0?sum/total:0);
  printf("requests=%d ok_2xx=%d avg_sec=%.3f max_sec=%.3f\n", total, ok, avg, max);
}
' "$TMP_FILE"
