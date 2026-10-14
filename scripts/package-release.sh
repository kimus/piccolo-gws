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
cleanup() {
  chmod -R u+w "$work" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT

mkdir -p "$work/bundle/bin" "$work/bundle/share/piccolo/integration" "$out_dir"

# Build both the integration wrapper and delegated backend with Nix, then
# include their runtime closure. The PicoClaw image is Alpine/musl; standalone
# glibc-linked Linux binaries fail there with "No such file or directory"
# because their ELF interpreter is absent.
piccolo_result="$work/piccolo-gws-result"
gws_result="$work/gws-result"
nix build --out-link "$piccolo_result" .#

# The delegated backend is intentionally packaged without Google Cloud SDK.
# If nixpkgs does not provide `gws` for this platform, release packaging must fail.
nix build --out-link "$gws_result" nixpkgs#gws
if [[ ! -x "$piccolo_result/bin/piccolo-gws" ]]; then
  echo "flake package did not provide an executable bin/piccolo-gws" >&2
  exit 1
fi
if [[ ! -x "$gws_result/bin/gws" ]]; then
  echo "nixpkgs#gws did not provide an executable bin/gws" >&2
  exit 1
fi
install -m 0755 "$piccolo_result/bin/piccolo-gws" "$work/bundle/bin/piccolo-gws"
install -m 0755 "$gws_result/bin/gws" "$work/bundle/bin/gws"

while IFS= read -r store_path; do
  mkdir -p "$work/bundle$(dirname "$store_path")"
  cp -a "$store_path" "$work/bundle$store_path"
done < <(nix-store -qR "$piccolo_result" "$gws_result" | sort -u)

# Grammar guard: every backend method this build maps must exist in the pinned
# gws version, otherwise tools fail at runtime with "unrecognized subcommand".
while IFS= read -r method; do
  if ! "$work/bundle/bin/gws" schema "$method" >/dev/null 2>&1; then
    echo "release bundle gws no longer provides method: $method" >&2
    exit 1
  fi
done < <("$work/bundle/bin/piccolo-gws" backend-methods)

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

# The descriptor must advertise the declarative setup surface Piccolo renders,
# and must not advertise host ports (browser callbacks ride Piccolo's courier).
export PATH="$work/bundle/bin:$PATH"
smoke_descriptor="$("$work/bundle/bin/piccolo-gws" describe --json)"
printf '%s' "$smoke_descriptor" | grep -q '"actions"' || {
  echo "release descriptor is missing the actions Piccolo renders" >&2
  exit 1
}
printf '%s' "$smoke_descriptor" | grep -q '"stateKeys"' || {
  echo "release descriptor is missing stateKeys" >&2
  exit 1
}
printf '%s' "$smoke_descriptor" | grep -q '"ports"' && {
  echo "release descriptor must not publish host ports for callbacks" >&2
  exit 1
}

# `state` must answer with the facts Piccolo gates its menu on.
"$work/bundle/bin/piccolo-gws" state --config-dir "$work/state" \
  | grep -q '"piccolo.integration.state.v1"' || {
  echo "state did not emit a piccolo.integration.state.v1 payload" >&2
  exit 1
}

# An unknown action id must be a structured error event, not prose.
set +e
unknown_event="$(printf '{}' | "$work/bundle/bin/piccolo-gws" call __nope__ --config-dir "$work/state" 2>/dev/null)"
set -e
printf '%s' "$unknown_event" | grep -q '"type":"error"' || {
  echo "call did not emit a structured error event for an unknown action" >&2
  exit 1
}

set +e
status_output="$(piccolo-gws status --config-dir "$work/state" --json 2>/dev/null)"
status_code=$?
set -e
if [[ $status_code -eq 0 ]]; then
  :
fi
printf '%s' "$status_output" | grep -q '"schema": "piccolo.integration.status.v1"'

( cd "$work/bundle" && tar -czf "$OLDPWD/$out_dir/$asset" . )
sha256sum "$out_dir/$asset" > "$out_dir/$asset.sha256"
printf '%s\n' "$out_dir/$asset"
