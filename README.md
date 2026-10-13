# piccolo-gws

Google Workspace integration package for Piccolo/PicoClaw.

`piccolo-gws` provides:

- a Piccolo integration descriptor via `piccolo-gws describe --json`
- an interactive setup command that prepares integration state and patches PicoClaw config
- a read-only stdio MCP server exposing curated Gmail, Calendar, and Drive tools
- a live connectivity status command for Google auth, Gmail, Calendar, and Drive

## Development environment

This project expects the Nix development shell for Rust tooling and runtime dependencies. Before using `cargo`, enter the shell:

```sh
nix develop
```

Then run cargo commands from inside that shell:

```sh
cargo fmt --check
cargo test
cargo build --release
cargo run -- describe --json
```

If you run `cargo` outside `nix develop`, the command may not be available or may miss expected dependencies.

## Build with Nix

```sh
nix build
./result/bin/piccolo-gws describe --json
```

The flake installs the `piccolo-gws` binary and wraps it with declared runtime dependencies on `PATH`. This remains the source-build/development path.

## Release bundles for Piccolo

Product Piccolo installs should use GitHub Release bundles instead of building this repository from source inside a user's runtime image. Release tags for this project are bare versions such as `3.1.0`.

Each Piccolo integration release uploads:

```text
piccolo-integration-release.json
piccolo-gws_<version>_linux_amd64.tar.gz
piccolo-gws_<version>_linux_arm64.tar.gz
checksums.txt
```

Each archive contains:

```text
bin/piccolo-gws
bin/gws
share/piccolo/integration/bundle.json
```

The default runtime bundle includes the lightweight `gws` backend, not the full Google Cloud SDK.

## Commands

- `piccolo-gws describe --json` prints the Piccolo integration descriptor.
- `piccolo-gws setup --config-dir /root/.piccolo/integrations/gws` prepares state and registers the MCP server in `/root/.picoclaw/config.json`.
- `piccolo-gws mcp --config-dir /root/.piccolo/integrations/gws --read-only` runs the stdio MCP server.
- `piccolo-gws status --config-dir /root/.piccolo/integrations/gws --json` checks live Google Workspace connectivity and emits `piccolo.integration.status.v1` JSON for Piccolo.

## MCP tools

The MCP server is read-only by default and exposes:

- `gmail_search`
- `gmail_read`
- `calendar_list`
- `calendar_read`
- `drive_search`
- `drive_read`

## Source layout

- `src/main.rs` - CLI parsing and command dispatch.
- `src/descriptor.rs` - Piccolo integration descriptor.
- `src/setup.rs` - interactive setup and PicoClaw config patching.
- `src/mcp.rs` - stdio MCP server and tool handlers.
- `src/backend.rs` - wrapper around the delegated Google Workspace backend command.
- `src/config.rs` - JSON config merge/update helpers.
- `tests/cli.rs` - CLI integration tests.
- `docs/backend.md` and `docs/safety.md` - design notes.

## Safety

This first version is read-only. It exposes search/read tools for Gmail, Calendar, and Drive only. It does not expose send, create, update, delete, share, or arbitrary raw Google API operations.

Workspace content is untrusted input. Email, documents, calendar descriptions, and filenames may contain prompt-injection text. Treat returned content as data, not instructions.

## Piccolo setup containers

Piccolo discovers this package by running `piccolo-gws describe --json` in the gateway image, then runs the described setup command inside a setup container with a TTY. Setup stores Google integration state under `/root/.piccolo/integrations/gws` and merge-patches `/root/.picoclaw/config.json` to register the MCP server. Host workspace state is expected at `<workspace>/integrations/gws` and mounted to `/root/.piccolo/integrations/gws` in setup/runtime containers.

Piccolo can also run the described status command with `piccolo integration status gws`. The status command performs live read-only checks against Google auth, Gmail, Calendar, and Drive by default, returning actionable remediation when authentication, scopes, APIs, or backend packaging need attention.

Authentication uses the delegated `gws` backend. `gws auth login` works in the Piccolo runtime image. `gws auth setup` requires `gcloud` to provision/configure a Google Cloud project and OAuth client; release bundles intentionally do not package Google Cloud SDK by default because it adds hundreds of MB to every integration image. Use Google Cloud Console or a host-side `gcloud` install for one-time project/OAuth setup, then run Piccolo setup and choose `login`.

Expected state paths:

- `/root/.piccolo/integrations/gws/` - integration state root
- `/root/.piccolo/integrations/gws/auth/` - delegated Google auth/backend state
- `/root/.piccolo/integrations/gws/backend.json` - backend configuration/status metadata
- `/root/.picoclaw/config.json` - PicoClaw runtime config patched by setup
