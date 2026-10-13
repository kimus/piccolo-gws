## Why

Piccolo is moving to a structured workspace layout where `/root/.picoclaw` is reserved for PicoClaw home data and Piccolo-managed integration state is mounted under `/root/.piccolo`. `piccolo-gws` must follow that ownership boundary and use an integration-name-based state directory.

## What Changes

- Change the default container config/state path from `/root/.picoclaw/integrations/google` to `/root/.piccolo/integrations/gws`.
- Update `describe --json` setup and MCP command metadata to use the new path.
- Update setup, MCP backend, config patching, docs, tests, and error messages to use `gws` as the state directory name.
- Keep host state expectation aligned with Piccolo: `<workspace>/integrations/gws` mounted to `/root/.piccolo/integrations/gws`.

## Capabilities

### New Capabilities

### Modified Capabilities
- `google-workspace-integration`: Update default state path and descriptor contract to use Piccolo-owned integration state under `/root/.piccolo/integrations/gws`.

## Impact

- Existing test workspaces using `/root/.picoclaw/integrations/google` will need to rerun integration setup after Piccolo's workspace-layout migration.
- Piccolo's generated MCP server config will point to the new `--config-dir` path.
- Documentation and tests must stop referencing `integrations/google`.
