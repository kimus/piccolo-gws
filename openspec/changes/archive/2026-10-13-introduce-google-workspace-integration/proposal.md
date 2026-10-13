## Why

Piccolo needs Google Workspace support that does not depend on an LLM discovering raw Google APIs or CLI behavior. A dedicated `piccolo-gws` integration can expose curated, deterministic Google Workspace MCP tools while owning Google-specific setup, authentication, and PicoClaw configuration changes outside Piccolo core.

## What Changes

- Introduce a Rust binary named `piccolo-gws`.
- Package `piccolo-gws` as an installable Nix flake package suitable for `github:kimus/piccolo-gws`.
- Add `piccolo-gws describe --json` for Piccolo integration discovery.
- Add `piccolo-gws setup` for interactive/provider-specific setup that runs inside a Piccolo setup container.
- Make setup own Google auth state and PicoClaw `config.json` patching for the Google Workspace MCP server.
- Add `piccolo-gws mcp` as a stdio MCP server exposing an initial read-only Google Workspace tool surface.
- Use `gws` / Google Workspace backend functionality internally instead of exposing raw `gws` behavior to the model.

## Capabilities

### New Capabilities
- `google-workspace-integration`: Self-describing Google Workspace integration package with setup, auth, config patching, and curated MCP tools.

### Modified Capabilities

## Impact

- New project/repo: `piccolo-gws`.
- Runtime dependency on Google Workspace APIs and OAuth credentials.
- Nix flake/package output for container installation.
- MCP server contract consumed by PicoClaw through Piccolo-managed gateway images.
- Security-sensitive handling of Workspace email, calendar, and drive content.
