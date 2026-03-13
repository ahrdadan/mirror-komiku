#!/bin/sh
set -eu

# Keep backend internal to container; nginx proxies /api to this address.
export BIND_ADDR="${BIND_ADDR:-127.0.0.1:8080}"

/usr/local/bin/mirror-komiku-proxy &
exec nginx -g "daemon off;"
