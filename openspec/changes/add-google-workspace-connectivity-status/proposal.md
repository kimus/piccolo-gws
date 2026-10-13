## Why

`piccolo-gws` can set up Google Workspace MCP tools, but users need a direct way to verify that the integration can actually connect to Google services. Status should test real Gmail, Calendar, and Drive connectivity with minimal read-only probes and return actionable remediation when a service is not available.

## What Changes

- Add `piccolo-gws status --json` to emit `piccolo.integration.status.v1` results.
- Default status checks perform live read-only connectivity probes for auth, Gmail, Calendar, and Drive.
- Add optional config path support so the command can also verify the PicoClaw MCP registration.
- Add a status command to `piccolo-gws describe --json` under `commands.status`.
- Return stable check IDs and remediation messages for login-required, missing scopes, disabled APIs, backend missing, and timeouts.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `google-workspace-integration`: Adds live connectivity status checks and descriptor status-command metadata.

## Impact

- CLI: new `status` subcommand.
- Descriptor: adds `commands.status` metadata using `/root/.piccolo/integrations/gws`.
- Backend: adds minimal read-only service probes for Gmail, Calendar, and Drive.
- Tests: CLI JSON shape, descriptor metadata, mocked backend status outcomes, and stable remediation text.
