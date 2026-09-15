#!/usr/bin/env bash
# Test the packaged binary in older Linux userspaces, including DNS and HTTPS.
# Requires Docker, Python 3 and OpenSSL. Containers share the Docker host kernel.
set -euo pipefail

binary=${1:?path to a native Linux release binary is required}
binary=$(cd "$(dirname "$binary")" && pwd)/$(basename "$binary")
test -f "$binary"
fixture=$(mktemp -d)
network="foxglove-release-smoke-$$"
server="$network-server"
client="$network-client"
cleanup() {
  docker rm -f "$client" >/dev/null 2>&1 || true
  docker rm -f "$server" >/dev/null 2>&1 || true
  docker network rm "$network" >/dev/null 2>&1 || true
  rm -rf "$fixture"
}
trap cleanup EXIT

# Use a private CA and local fixture, without production requests or credentials.
openssl req -x509 -newkey rsa:2048 -nodes -days 1 \
  -keyout "$fixture/ca-key.pem" -out "$fixture/ca.pem" \
  -subj '/CN=Foxglove smoke CA' -addext 'basicConstraints=critical,CA:TRUE' >/dev/null 2>&1
openssl req -new -newkey rsa:2048 -nodes \
  -keyout "$fixture/key.pem" -out "$fixture/server.csr" \
  -subj '/CN=fixture' >/dev/null 2>&1
cat > "$fixture/extensions.cnf" <<'EXTENSIONS'
basicConstraints=critical,CA:FALSE
subjectAltName=DNS:fixture
extendedKeyUsage=serverAuth
EXTENSIONS
openssl x509 -req -in "$fixture/server.csr" -CA "$fixture/ca.pem" \
  -CAkey "$fixture/ca-key.pem" -CAcreateserial -days 1 \
  -extfile "$fixture/extensions.cnf" -out "$fixture/server.pem" >/dev/null 2>&1
cat > "$fixture/config.yaml" <<'CONFIG'
base_url: https://fixture:8443
bearer_token: smoke-test
CONFIG
cat > "$fixture/server.py" <<'PY'
import http.server
import ssl
from urllib.parse import urlsplit


class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if urlsplit(self.path).path != "/v1/recordings" or self.headers.get("Authorization") != "Bearer smoke-test":
            self.send_error(400)
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", "2")
        self.end_headers()
        self.wfile.write(b"[]")


server = http.server.HTTPServer(("0.0.0.0", 8443), Handler)
context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.load_cert_chain("/fixture/server.pem", "/fixture/key.pem")
server.socket = context.wrap_socket(server.socket, server_side=True)
server.serve_forever()
PY

docker network create --internal "$network" >/dev/null
docker run --detach --name "$server" --network "$network" --network-alias fixture \
  --mount "type=bind,src=$fixture,dst=/fixture,readonly" \
  python:3.12-slim python /fixture/server.py >/dev/null
docker exec "$server" python -c '
import socket, time
for attempt in range(100):
    try:
        socket.create_connection(("127.0.0.1", 8443), timeout=1).close()
        break
    except OSError:
        time.sleep(0.1)
else:
    raise SystemExit("HTTPS fixture did not start")
'

# Compatibility baselines, not a claim about vendors' changing support periods.
for distro in ubuntu:18.04 ubuntu:20.04 ubuntu:22.04 debian:12 rockylinux:8 opensuse/leap:15.6 alpine:3.22; do
  echo "Testing $binary on $distro"
  docker pull "$distro"
  # Pull outside the request deadline. Cleanup stops the container on timeout.
  python3 -c 'import subprocess, sys; sys.exit(subprocess.run(sys.argv[1:], timeout=120).returncode)' \
    docker run --rm --name "$client" --network "$network" \
    --mount "type=bind,src=$binary,dst=/usr/local/bin/foxglove,readonly" \
    --mount "type=bind,src=$fixture,dst=/fixture,readonly" \
    --env SSL_CERT_FILE=/fixture/ca.pem \
    "$distro" sh -ec '
      foxglove --help >/dev/null
      foxglove version
      result=$(foxglove --config /fixture/config.yaml recordings list --format json)
      test "$result" = "[]"
    '
done
