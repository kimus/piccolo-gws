#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <version> <out-dir>" >&2
  exit 2
fi

version="$1"
out_dir="$2"
package="github:kimus/piccolo-gws"

case "$(uname -m)" in
  x86_64) arch="amd64" ;;
  aarch64|arm64) arch="arm64" ;;
  *) echo "unsupported architecture: $(uname -m)" >&2; exit 2 ;;
esac
platform="linux/${arch}"
asset="piccolo-gws_${version}_linux_${arch}.tar.gz"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

mkdir -p "$work/bundle/bin" "$work/bundle/share/piccolo/integration" "$out_dir"

cargo build --release --locked
install -m 0755 target/release/piccolo-gws "$work/bundle/bin/piccolo-gws"

# The delegated backend is intentionally packaged without Google Cloud SDK.
# If nixpkgs does not provide `gws` for this platform, release packaging must fail.
gws_result="$work/gws-result"
nix build --out-link "$gws_result" nixpkgs#gws
if [[ ! -x "$gws_result/bin/gws" ]]; then
  echo "nixpkgs#gws did not provide an executable bin/gws" >&2
  exit 1
fi
install -m 0755 "$gws_result/bin/gws" "$work/bundle/bin/gws"

if [[ -e "$work/bundle/bin/gcloud" ]] || find "$work/bundle" -type f -name gcloud | grep -q .; then
  echo "release bundle must not contain gcloud" >&2
  exit 1
fi

cat > "$work/bundle/share/piccolo/integration/bundle.json" <<JSON
{
  "schema": "piccolo.integration.bundle.v1",
  "package": "${package}",
  "version": "${version}",
  "platform": "${platform}",
  "commands": ["piccolo-gws", "gws"]
}
JSON

PATH="$work/bundle/bin:$PATH" "$work/bundle/bin/piccolo-gws" describe --json | grep -q '"schema": "piccolo.integration.v1"'
set +e
status_output="$(PATH="$work/bundle/bin:$PATH" "$work/bundle/bin/piccolo-gws" status --config-dir "$work/state" --json 2>/dev/null)"
status_code=$?
set -e
if [[ $status_code -eq 0 ]]; then
  :
fi
printf '%s' "$status_output" | grep -q '"schema": "piccolo.integration.status.v1"'

( cd "$work/bundle" && tar -czf "$OLDPWD/$out_dir/$asset" . )
sha256sum "$out_dir/$asset" > "$out_dir/$asset.sha256"
printf '%s\n' "$out_dir/$asset"
