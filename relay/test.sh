#!/usr/bin/env bash
# Runs the relay locally (`wrangler dev`, no Cloudflare account needed) and
# the tests in test/ against it. Needs Node.js 22 or newer (for its built-in
# WebSocket) and wrangler on PATH, e.g.
#   nix shell nixpkgs#nodejs nixpkgs#wrangler -c ./test.sh
set -euo pipefail
cd "$(dirname "$0")"

port=${PORT:-8790}
export WRANGLER_SEND_METRICS=false
wrangler dev --ip 127.0.0.1 --port "$port" --log-level warn &
pid=$!
trap 'kill $pid 2>/dev/null; wait $pid 2>/dev/null || true' EXIT

# Wait until it answers (a plain GET outside the relay's paths gets a 404).
node -e '
const url = process.argv[1];
const until = Date.now() + 60000;
(async () => {
  for (;;) {
    try { await fetch(url); process.exit(0); } catch {}
    if (Date.now() > until) { console.error("wrangler dev did not start"); process.exit(1); }
    await new Promise((r) => setTimeout(r, 250));
  }
})();
' "http://127.0.0.1:$port/"

RELAY="ws://127.0.0.1:$port" node --test --test-concurrency=1 test/*.test.mjs
