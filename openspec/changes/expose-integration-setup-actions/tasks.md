# Tasks: expose-integration-setup-actions

## 1. Descriptor actions

- [ ] 1.1 Add an `actions` list to `describe` output: credential setup, browser authorization, connectivity diagnosis
- [ ] 1.2 Give every action a stable opaque id, a `setup`/`diagnostic` role, a title, and typed inputs from the closed set
- [ ] 1.3 Declare the state fact keys and reference them from each action's `requires`
- [ ] 1.4 Declare a `path` input for the OAuth client secret with a state-relative destination and sensitive marking
- [ ] 1.5 Keep the schema identifier and the existing `setup.command` populated for older Piccolo builds
- [ ] 1.6 Update descriptor tests for the new fields while asserting the unchanged schema identifier

## 2. State facts

- [ ] 2.1 Implement `piccolo-gws state` printing JSON on stdout with no required flag
- [ ] 2.2 Report credentials, login, and provider-API facts from `backend_auth_state` rather than exit codes
- [ ] 2.3 Report a token without usable credentials as not-authorized instead of configured
- [ ] 2.4 Unit-test each fact combination (none, credentials only, token only, both)

## 3. Action invocation

- [ ] 3.1 Implement `piccolo-gws call <action-id>` taking one static subcommand with an id argument
- [ ] 3.2 Read answers as a single JSON document from stdin; accept no answer value from argv or env
- [ ] 3.3 Emit newline-delimited events on stdout and route human logs to stderr
- [ ] 3.4 Define one event envelope with a schema identifier and the `step`/`open_url`/`wait_redirect`/`result`/`error` types
- [ ] 3.5 Emit an error event and exit non-zero for an unknown id or an unmet prerequisite, with no side effects
- [ ] 3.6 Guarantee no secret value appears in any event, log line, or error message

## 4. Console links

- [ ] 4.1 Emit `open_url` with a one-line label for Google Cloud Console steps instead of prose instructions
- [ ] 4.2 Build project-deep-linked URLs from the stored `installed.project_id` when present, falling back to the console root
- [ ] 4.3 Cover URL construction with tests for the project-present and project-absent cases

## 5. Browser callback relay

- [ ] 5.1 Emit `wait_redirect` carrying the full authorization URL and keep the container-side listener alive
- [ ] 5.2 Complete the exchange from the request path and query the host relays into loopback
- [ ] 5.3 Bound the wait with an internal timeout, emit an error event, and close the listener on expiry
- [ ] 5.4 Make the login path testable without a terminal by relaying a callback from a test

## 6. Stop writing runtime config

- [ ] 6.1 Remove `merge_patch`, `backup_path`, `write_config_with_backup`, `summarize_patch`, and `mcp_server_patch` from `src/config.rs`
- [ ] 6.2 Return the MCP registration as `result.configPatch` in the shape Piccolo's allowlist accepts
- [ ] 6.3 Ensure no code path creates, replaces, or backs up `config.json`
- [ ] 6.4 Migrate credential state to the file Piccolo stages at the declared destination, read by container path
- [ ] 6.5 Update tests that asserted self-patching to assert patch proposal instead

## 7. Retire the in-binary wizard

- [ ] 7.1 Remove the guided menu and pasted-secret reader from `src/setup.rs`
- [ ] 7.2 Drop the `inquire` dependency from `Cargo.toml`/lockfile
- [ ] 7.3 Keep `setup` as a thin non-interactive entry so scripted and headless use still works
- [ ] 7.4 Reconcile `add-guided-google-workspace-setup` so its menu requirements are marked superseded before archiving

## 8. Verification

- [ ] 8.1 Test `call` end to end: answers on stdin, events on stdout, logs on stderr, correct exit codes
- [ ] 8.2 Test that no secret material reaches argv, env, events, or logs
- [ ] 8.3 Update `scripts/package-release.sh` smoke checks for `state` and `call` alongside `describe`
- [ ] 8.4 Run `nix develop -c cargo fmt --check`, `nix develop -c cargo clippy --all-targets -- -D warnings`, `nix develop -c cargo test`, `nix build .#`
- [ ] 8.5 Release an integration version and verify through `piccolo integration add` → `setup`
- [ ] 8.6 Update README: Piccolo renders setup, this package declares actions and proposes config, and the wizard/config-writing code is gone
