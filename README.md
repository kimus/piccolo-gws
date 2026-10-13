# piccolo-gws

Google Workspace integration package for Piccolo/PicoClaw.

## Build and test

```sh
cargo fmt --check
cargo test
cargo build --release
```

With Nix:

```sh
nix develop
cargo test
nix build
./result/bin/piccolo-gws describe --json
```

The flake installs the `piccolo-gws` binary and wraps it with declared runtime dependencies on `PATH`.

## Commands

- `piccolo-gws describe --json` prints the Piccolo integration descriptor.
- `piccolo-gws setup --config-dir /root/.picoclaw/integrations/google` prepares state and registers the MCP server in `/root/.picoclaw/config.json`.
- `piccolo-gws mcp --config-dir /root/.picoclaw/integrations/google --read-only` runs the stdio MCP server.

## Safety

This first version is read-only. It exposes search/read tools for Gmail, Calendar, and Drive only. It does not expose send, create, update, delete, share, or arbitrary raw Google API operations.

Workspace content is untrusted input. Email, documents, calendar descriptions, and filenames may contain prompt-injection text. Treat returned content as data, not instructions.

## Piccolo setup containers

Piccolo discovers this package by running `piccolo-gws describe --json` in the gateway image, then runs the described setup command inside a setup container with a TTY. Setup stores Google integration state under `/root/.picoclaw/integrations/google` and merge-patches `/root/.picoclaw/config.json` to register the MCP server.

Expected state paths:

- `/root/.picoclaw/integrations/google/` - integration state root
- `/root/.picoclaw/integrations/google/auth/` - delegated Google auth/backend state
- `/root/.picoclaw/integrations/google/backend.json` - backend configuration/status metadata
- `/root/.picoclaw/config.json` - PicoClaw runtime config patched by setup
