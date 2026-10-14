## Context

Verified against `gws` 0.22.5 inside the runtime image:

```text
gws <service> <resource> [sub-resource] <method> [flags]
  gws gmail users messages list --params '{"userId":"me"}'
  gws calendar events list --params '{"calendarId":"primary"}'
  gws drive files list --params '{"pageSize":10}'
  gws schema <service.resource.method>     # parameter contract
  gws ... --dry-run                        # validates locally, no auth needed
```

Required parameters (from `parameterOrder` in `gws schema`):

| Method | Required |
| --- | --- |
| `gmail.users.messages.list` | `userId` |
| `gmail.users.messages.get` | `userId`, `id` |
| `calendar.events.list` | `calendarId` |
| `calendar.events.get` | `calendarId`, `eventId` |
| `drive.files.list` | (none) |
| `drive.files.get` | `fileId` |

Note the asymmetry: Calendar read takes `eventId`, not `id`, and Gmail always needs `userId`.

## Goals / Non-Goals

**Goals:**
- All six read-only tools return real API data through the delegated backend.
- Status probes measure actual service reachability instead of CLI parse errors.
- Mismatch is caught before release, not by an end user.

**Non-Goals:**
- No new tools, no write operations, no helper commands (`+send`, `+triage`).
- No change to the MCP surface exposed to PicoClaw (tool names/shape stay).

## Decisions

- **`--params` JSON over CLI flags** — the real CLI takes parameters as one JSON object; building it with `serde_json::json!` avoids shell quoting and flag drift between versions.
- **Sensible required defaults** — Gmail `userId: "me"`, Calendar `calendarId: "primary"`. Alternatives (prompting the user, exposing in config) add surface for little gain; the defaults match the read-only assistant use case. Expose optional overrides via tool args where cheap.
- **Keep the tool-name → argv mapping in one function** — `backend.rs` stays the single place that knows `gws` grammar, so status and MCP both inherit correctness.
- **Release-time grammar check** — `scripts/package-release.sh` runs `gws schema` for each mapped method plus `--dry-run` on one call, so a `gws` version bump that renames a resource fails packaging rather than the user.

## Risks / Trade-offs

- [`gws` schema drifts across versions] → the packaging guard pins expectations; failure message names the method that disappeared.
- [`calendarId`/`userId` defaults may not fit every user] → accepted for read-only v1; overrides can be added to tool args later.
- [`--dry-run` behavior may change] → used only as a smoke check, never as an auth substitute.
