## Why

A live connectivity check printed `[fail] drive: Using keyring backend: keyring`. The backend writes an informational banner to stderr and puts the actual cause on stdout as JSON — code, reason, message and a ready-made enable URL. `run_gws` bailed with the stderr blob on any non-zero exit, so the banner became the user's error and the provider's own fix was discarded.

The classifier made it worse: it searched that prose for `auth`, `login` or `credential`, and `"authorization"` contains `"auth"` — so a disabled API or a scope problem could be reported as "log in again", sending the user to reauthorize something that was fine.

## What Changes

- Failed backend calls are read as a structured envelope first: `reason`, `message`, `enable_url`, HTTP code. Raw stderr is the fallback only when no envelope exists.
- `login_required` becomes a structured decision, never a text search.
- `check_connectivity` counts what failed, names the services, and passes the enable link through as `remediationUrl`.
- `status` carries `remediationLabel`/`remediationUrl` so `piccolo integration status` can show the link.
- An escaped error can no longer exit silently: `main.rs` emits an `internal` error event before exiting, because the contract is that the stream always ends with a terminal event and silence reads as a crash.
- `login_required_for` stopped printing a container-side path the user cannot type.

## Capabilities

### New Capabilities

### Modified Capabilities

- `google-workspace-integration`: failure reporting derives from the provider's structured error, and every exit is announced on the event stream

## Impact

- `src/backend.rs` — `ToolError` gains `reason`/`enable_url`; `parse_backend_failure`, `extract_json_object`, downcast-preserving propagation
- `src/actions.rs` — connectivity summary and link
- `src/status.rs` — remediation fields surfaced
- `src/main.rs` — guaranteed terminal event
- Tests: real `accessNotConfigured` fixture, a 401 case, prose containing "authorization" asserting **no** login prompt, and banner-mixed stdout
