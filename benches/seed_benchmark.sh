#!/usr/bin/env bash
# Benchmark script for Emberwake performance validation (T080 / SC-001..004).
# Seeds 200 services + 500 bookmarks, then measures SSR response time.
# Requires a running Emberwake server on localhost:5005.
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:5005}"

echo "=== Emberwake Performance Benchmark ==="
echo "Target: ${BASE_URL}"
echo

# Check server is up.
if ! curl -sf "${BASE_URL}/readyz" >/dev/null 2>&1; then
  echo "ERROR: Server not ready at ${BASE_URL}/readyz"
  echo "Start the server first: cargo leptos watch"
  exit 1
fi

echo "--- Step 1: First-run setup (admin) ---"
# Check if setup is already done.
SETUP_STATUS=$(curl -s -o /dev/null -w '%{http_code}' "${BASE_URL}/setup")
if [ "$SETUP_STATUS" = "200" ]; then
  curl -sf -X POST "${BASE_URL}/api/setup" \
    -H 'Content-Type: application/json' \
    -d '{"username":"admin","password":"benchmark-pass-123","email":"admin@bench.local"}' \
    -c /tmp/emberwake_bench_cookies.txt || true
else
  echo "Setup already complete, logging in..."
  curl -sf -X POST "${BASE_URL}/api/login" \
    -H 'Content-Type: application/json' \
    -d '{"username":"admin","password":"benchmark-pass-123"}' \
    -c /tmp/emberwake_bench_cookies.txt
fi

echo "--- Step 2: Seed 200 services + 500 bookmarks ---"
# Create categories.
for i in $(seq 1 10); do
  curl -sf -X POST "${BASE_URL}/api/v1/categories" \
    -H 'Content-Type: application/json' \
    -b /tmp/emberwake_bench_cookies.txt \
    -d "{\"name\":\"Category ${i}\",\"position\":${i}}" >/dev/null 2>&1 || true
done

# Create 200 services (20 per category).
for i in $(seq 1 200); do
  cat=$(( (i - 1) / 20 + 1 ))
  curl -sf -X POST "${BASE_URL}/api/v1/services" \
    -H 'Content-Type: application/json' \
    -b /tmp/emberwake_bench_cookies.txt \
    -d "{\"name\":\"Service ${i}\",\"url\":\"https://svc${i}.local\",\"category_id\":${cat},\"position\":${i}}" >/dev/null 2>&1 || true
done

# Create 500 bookmarks.
for i in $(seq 1 500); do
  cat=$(( (i - 1) / 50 + 1 ))
  curl -sf -X POST "${BASE_URL}/api/v1/bookmarks" \
    -H 'Content-Type: application/json' \
    -b /tmp/emberwake_bench_cookies.txt \
    -d "{\"title\":\"Bookmark ${i}\",\"url\":\"https://bm${i}.local\",\"category_id\":${cat}}" >/dev/null 2>&1 || true
done

echo "Seeded 200 services + 500 bookmarks."
echo

echo "--- Step 3: Measure SSR TTFB (SC-001) ---"
# Warm up.
curl -sf "${BASE_URL}/" -b /tmp/emberwake_bench_cookies.txt >/dev/null 2>&1 || true

# Measure TTFB 20 times.
TTFB_TIMES=""
for i in $(seq 1 20); do
  TTFB=$(curl -sf -o /dev/null -w '%{time_starttransfer}' "${BASE_URL}/" -b /tmp/emberwake_bench_cookies.txt 2>/dev/null || echo "0")
  TTFB_TIMES="${TTFB_TIMES} ${TTFB}"
done
echo "TTFB samples (seconds): ${TTFB_TIMES}"
echo "TTFB avg: $(echo "${TTFB_TIMES}" | tr ' ' '\n' | grep -v '^$' | awk '{s+=$1; n++} END {printf "%.4f\n", s/n}')s"
echo "TTFB p50: $(echo "${TTFB_TIMES}" | tr ' ' '\n' | grep -v '^$' | sort -n | awk 'NR==10 || NR==11 {print; exit}')s"
echo "TTFB p95: $(echo "${TTFB_TIMES}" | tr ' ' '\n' | grep -v '^$' | sort -n | awk 'NR==19 {print; exit}')s"
echo

echo "--- Step 4: Measure total response time (SC-001 interactive proxy) ---"
TOTAL_TIMES=""
for i in $(seq 1 20); do
  TOTAL=$(curl -sf -o /dev/null -w '%{time_total}' "${BASE_URL}/" -b /tmp/emberwake_bench_cookies.txt 2>/dev/null || echo "0")
  TOTAL_TIMES="${TOTAL_TIMES} ${TOTAL}"
done
echo "Total response avg: $(echo "${TOTAL_TIMES}" | tr ' ' '\n' | grep -v '^$' | awk '{s+=$1; n++} END {printf "%.4f\n", s/n}')s"
echo

echo "--- Step 5: Measure CRUD server-function p95 (SC-002) ---"
# Measure create service latency.
CRUD_TIMES=""
for i in $(seq 1 50); do
  CRUD=$(curl -sf -o /dev/null -w '%{time_total}' -X POST "${BASE_URL}/api/v1/services" \
    -H 'Content-Type: application/json' \
    -b /tmp/emberwake_bench_cookies.txt \
    -d "{\"name\":\"Bench Svc ${i}\",\"url\":\"https://bench${i}.local\",\"category_id\":1,\"position\":${i}}" 2>/dev/null || echo "0")
  CRUD_TIMES="${CRUD_TIMES} ${CRUD}"
done
echo "CRUD create avg: $(echo "${CRUD_TIMES}" | tr ' ' '\n' | grep -v '^$' | awk '{s+=$1; n++} END {printf "%.4f\n", s/n}')s"
echo "CRUD create p95: $(echo "${CRUD_TIMES}" | tr ' ' '\n' | grep -v '^$' | sort -n | awk 'NR==47 || NR==48 {print; exit}')s"
echo

echo "--- Step 6: Measure idle RSS (SC-003) ---"
if command -v pidof >/dev/null 2>&1; then
  PID=$(pidof emberwake 2>/dev/null || echo "")
  if [ -n "$PID" ]; then
    RSS=$(grep VmRSS /proc/$PID/status 2>/dev/null | awk '{print $2}')
    echo "Idle RSS: ${RSS} kB ($(echo "scale=1; ${RSS}/1024" | bc 2>/dev/null || echo "N/A") MB)"
  fi
fi
echo

echo "--- Step 7: WASM bundle size (SC-004) ---"
WASM_FILE=$(find target/site/pkg/ -name '*.wasm' 2>/dev/null | head -1)
JS_FILE=$(find target/site/pkg/ -name '*.js' 2>/dev/null | head -1)
CSS_FILE=$(find target/site/pkg/ -name '*.css' 2>/dev/null | head -1)
if [ -n "$WASM_FILE" ]; then
  WASM_RAW=$(stat -c%s "$WASM_FILE")
  WASM_GZIP=$(gzip -c "$WASM_FILE" | wc -c)
  JS_RAW=$(stat -c%s "$JS_FILE" 2>/dev/null || echo 0)
  JS_GZIP=$(gzip -c "$JS_FILE" 2>/dev/null | wc -c || echo 0)
  CSS_RAW=$(stat -c%s "$CSS_FILE" 2>/dev/null || echo 0)
  CSS_GZIP=$(gzip -c "$CSS_FILE" 2>/dev/null | wc -c || echo 0)
  TOTAL_GZIP=$((WASM_GZIP + JS_GZIP + CSS_GZIP))
  echo "WASM raw: ${WASM_RAW} bytes (${WASM_GZIP} bytes gzip)"
  echo "JS raw: ${JS_RAW} bytes (${JS_GZIP} bytes gzip)"
  echo "CSS raw: ${CSS_RAW} bytes (${CSS_GZIP} bytes gzip)"
  echo "Total compressed: ${TOTAL_GZIP} bytes ($((TOTAL_GZIP / 1024)) KB)"
  echo "Budget: 350 KB compressed"
  if [ $((TOTAL_GZIP / 1024)) -lt 350 ]; then
    echo "PASS: Within budget"
  else
    echo "FAIL: Exceeds budget"
  fi
fi
echo
echo "=== Benchmark complete ==="
