## 1. Descriptor and CLI

- [x] 1.1 Add `status` command metadata to `piccolo-gws describe --json` under `commands.status`.
- [x] 1.2 Add `piccolo-gws status` CLI subcommand.
- [x] 1.3 Add `--config-dir`, `--picoclaw-config`, and `--json` flags to the status command.
- [x] 1.4 Require `--json` for the initial status output contract or provide a clear error for unsupported human output.

## 2. Status schema and rendering

- [x] 2.1 Add Rust structs for `piccolo.integration.status.v1` output.
- [x] 2.2 Add check statuses `ok`, `warn`, `fail`, `unknown`, and `skip`.
- [x] 2.3 Derive overall status from check results.
- [x] 2.4 Include stable check IDs: `backend`, `state-dir`, `auth`, `gmail`, `calendar`, `drive`, and optional `picoclaw-config`.
- [x] 2.5 Include remediation fields for login-required, missing backend, missing scopes/API access, and timeouts.

## 3. Connectivity checks

- [x] 3.1 Check that the integration state directory and auth directory exist or can be inspected.
- [x] 3.2 Check that `gws` is available on PATH.
- [x] 3.3 Run `gws auth status` and map success/failure to the `auth` check.
- [x] 3.4 If auth fails, mark Gmail, Calendar, and Drive checks as `skip` with auth-dependent messages.
- [x] 3.5 Add a minimal read-only Gmail probe using the packaged `gws` backend.
- [x] 3.6 Add a minimal read-only Calendar probe using the packaged `gws` backend.
- [x] 3.7 Add a minimal read-only Drive probe using the packaged `gws` backend.
- [x] 3.8 Normalize backend errors into stable status messages and remediations.

## 4. PicoClaw config check

- [x] 4.1 When `--picoclaw-config` is provided, parse the PicoClaw config JSON.
- [x] 4.2 Verify `tools.mcp.servers.google_workspace.command == "piccolo-gws"`.
- [x] 4.3 Verify MCP server args include the selected `--config-dir` and read-only mode.
- [x] 4.4 Report config parse/missing/mismatch failures as actionable checks.

## 5. Tests and docs

- [x] 5.1 Update descriptor tests to assert `commands.status` and `/root/.piccolo/integrations/gws` path usage.
- [x] 5.2 Add CLI tests for `piccolo-gws status --json` output shape.
- [x] 5.3 Add mocked/backend-isolated tests for auth missing and backend missing outcomes.
- [x] 5.4 Add mocked/backend-isolated tests for Gmail, Calendar, and Drive success/failure mapping.
- [x] 5.5 Add tests for optional PicoClaw config validation.
- [x] 5.6 Document `piccolo-gws status --json` and how Piccolo runs it.
- [x] 5.7 Run the Rust test suite where `cargo` is available.
- [x] 5.8 Validate `add-google-workspace-connectivity-status` OpenSpec artifacts.
