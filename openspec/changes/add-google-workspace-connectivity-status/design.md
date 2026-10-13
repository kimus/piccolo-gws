## Context

`piccolo-gws` delegates Google auth to the packaged `gws` CLI and exposes read-only MCP tools by shelling out to `gws` commands. Setup already checks `gws auth status`, but there is no standalone status command and no service-specific connectivity check for Gmail, Calendar, or Drive.

Piccolo core should remain provider-neutral. Therefore `piccolo-gws` should own the meaning of Google Workspace health and emit the generic Piccolo integration status schema.

## Goals / Non-Goals

**Goals:**

- Provide `piccolo-gws status --json` for Piccolo to orchestrate.
- Test real connectivity to Google services by default.
- Keep probes minimal, read-only, and bounded by timeouts.
- Report separate checks for backend availability, auth, Gmail, Calendar, Drive, and optional PicoClaw config registration.
- Return actionable remediation for common failures.
- Advertise the status command in the integration descriptor.

**Non-Goals:**

- Do not add write-capable Google Workspace operations.
- Do not inspect or expose user data beyond minimal probe metadata.
- Do not make Piccolo core understand Google-specific statuses.
- Do not replace interactive setup; status can diagnose but not perform OAuth setup.

## Decisions

### Default to live connectivity checks

`piccolo-gws status --json` will perform live service probes by default because the product question is whether Gmail, Calendar, and Drive can connect successfully. A future local-only flag may be added, but the first version prioritizes real connectivity.

### Minimal read-only probes

Each service check should use the smallest safe read-only operation available through `gws`:

- Gmail: search with a restrictive query and low limit.
- Calendar: list events for a narrow window.
- Drive: search with a low limit.

The exact command arguments should avoid reading full document/message bodies and should rely on existing backend normalization where possible.

### Stable Piccolo status schema

The command emits:

```json
{
  "schema": "piccolo.integration.status.v1",
  "name": "gws",
  "status": "fail",
  "summary": "Calendar cannot connect",
  "checks": [
    {"id":"auth","label":"Google auth","status":"ok","message":"authenticated"},
    {"id":"calendar","label":"Calendar","status":"fail","message":"insufficient scopes","remediation":"Run piccolo integration setup gws"}
  ]
}
```

Allowed check statuses are `ok`, `warn`, `fail`, `unknown`, and `skip`.

### Skip dependent service checks when auth fails

If auth is missing or invalid, Gmail/Calendar/Drive checks should be reported as `skip` rather than producing three redundant failures. The remediation points to `piccolo integration setup gws`.

### Optional PicoClaw config check

Status can accept `--picoclaw-config /root/.picoclaw/config.json`. When provided, it verifies that the Google Workspace MCP server is configured with the expected command and config dir. This is useful in Piccolo one-shot status containers but keeps the command usable outside Piccolo.

## Risks / Trade-offs

- **Live checks can be slow or quota-sensitive** → Use low limits, read-only commands, and existing backend timeouts.
- **Different `gws` CLI output/errors may vary** → Normalize common errors into stable status codes and messages.
- **Scopes may be partial** → Service-specific checks make partial failures visible instead of treating all auth as one boolean.
- **Status may refresh OAuth tokens** → This is acceptable as backend-owned auth behavior; status remains read-only with respect to Workspace data.

## Open Questions

- What exact `gws` commands are safest for Gmail, Calendar, and Drive smoke checks?
- Should status include latency/duration fields now, or defer until needed?
