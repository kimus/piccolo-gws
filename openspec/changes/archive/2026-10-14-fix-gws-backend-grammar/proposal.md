## Why

Every `piccolo-gws` tool and status probe was written against an assumed `gws` grammar (`gmail search`, `calendar list`, `drive read`) that does not exist in the pinned `gws` 0.22.5. Real grammar is `gws <service> <resource> [sub-resource] <method> --params <JSON>`, so all six read-only tools fail with `unrecognized subcommand` and `piccolo integration status` reports three false failures even when auth is fine.

## What Changes

- Map each MCP tool to the verified `gws` resource/method path with `--params` JSON.
- Add required parameters the assumed grammar hid: `userId` for Gmail, `calendarId`/`eventId` for Calendar, `fileId` for Drive.
- Keep tool names, read-only defaults, and the login-required behavior unchanged.
- Status probes reuse the corrected tool paths so Gmail/Calendar/Drive checks reflect real API reachability.
- Add a packaging-time guard that rejects unknown `gws` subcommands rather than shipping a silent grammar mismatch.

## Capabilities

### New Capabilities

- `gws-backend-invocation`: How `piccolo-gws` invokes the delegated `gws` CLI, parameter requirements, and verification against the pinned version.

### Modified Capabilities

## Impact

- `src/backend.rs`: replace the argv builders with verified resource/method + `--params` shapes.
- `src/status.rs`: probe arguments align with the corrected tool mapping.
- `src/mcp.rs`: tool input schemas gain the parameters the real API requires.
- Tests: assert generated argv per tool; CI smoke runs `gws schema <resource.method>` for each mapped method.
