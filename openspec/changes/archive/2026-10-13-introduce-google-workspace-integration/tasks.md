## 1. Project foundation

- [x] Create Rust workspace/package for `piccolo-gws`.
- [x] Add CLI structure with `describe`, `setup`, and `mcp` subcommands.
- [x] Add Nix flake packaging that installs `piccolo-gws` and required runtime dependencies.
- [x] Add CI/build instructions for local development.

## 2. Integration descriptor

- [x] Implement `piccolo-gws describe --json`.
- [x] Define descriptor schema output for `piccolo.integration.v1`.
- [x] Include setup command, TTY requirement, setup port needs, and display metadata.
- [x] Add tests validating descriptor JSON shape.

## 3. Setup flow

- [x] Implement `piccolo-gws setup --config-dir <path>`.
- [x] Create and validate integration state directory.
- [x] Implement Google auth setup/status flow or delegate to backend safely.
- [x] Implement conservative merge-patching for `/root/.picoclaw/config.json`.
- [x] Create config backup before writes.
- [x] Show diff/summary and require confirmation before patching.
- [x] Register `tools.mcp.servers.google_workspace` with stdio command `piccolo-gws mcp ...`.

## 4. MCP server

- [x] Implement stdio MCP server entrypoint `piccolo-gws mcp`.
- [x] Define read-only Gmail search/read tools.
- [x] Define read-only Calendar list/read tools.
- [x] Define read-only Drive search/read tools.
- [x] Return normalized JSON/text results with stable error shapes.
- [x] Return clear login-required errors when auth is incomplete.

## 5. Backend integration

- [x] Decide initial backend: shell out to `gws` or use Rust Google Workspace APIs directly.
- [x] If shelling out, wrap command execution with timeouts, structured output parsing, and normalized errors.
- [x] Ensure backend dependency is included by the Nix flake package/wrapper.
- [x] Add tests with fake backend responses.

## 6. Safety and documentation

- [x] Document read-only default behavior and excluded write operations.
- [x] Document prompt-injection risks from email/docs/calendar content.
- [x] Document setup flow when run by Piccolo setup containers.
- [x] Document expected state paths under `/root/.picoclaw/integrations/google`.

## 7. Verification

- [x] Unit-test descriptor, config patching, and backend normalization.
- [x] Add integration test for `piccolo-gws mcp` startup over stdio.
- [x] Verify Nix flake install exposes `piccolo-gws` on PATH.
