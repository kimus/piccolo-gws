## Why

Piccolo now owns integration setup UI. `piccolo-gws` currently ships its own Rust `inquire` wizard, writes PicoClaw's `config.json` itself, and blocks on a browser callback that cannot reach its container loopback. That duplicates Piccolo's form rendering and config patching (including a divergent backup name, `config.json.bak.<epoch>` vs `config.json.bak`), and it cannot see the host filesystem to import a downloaded OAuth client.

## What Changes

- Add `piccolo-gws state` (JSON-only, no flag) reporting integration facts: credentials, login, provider API readiness.
- Add `piccolo-gws call <action-id>`: reads answers as one JSON document on **stdin**, emits newline-delimited events on **stdout**, human logs on stderr.
- Publish the action menu from `describe` while keeping the descriptor schema identifier unchanged, so older Piccolo builds still parse it.
- **Stop writing `config.json`.** Successful actions propose a patch inside `result`; Piccolo validates and applies it. The Rust merge-patch/backup/diff helpers are removed.
- Emit `open_url` for user steps that live in Google Cloud Console, and `wait_redirect` when browser authorization must complete.
- Read a staged `client_secret.json` from a container-side path supplied by Piccolo instead of pasting secrets into a terminal.
- **BREAKING** the interactive wizard surface: the `inquire` dependency and the in-binary menu are removed; a plain non-interactive CLI path remains.

## Capabilities

### New Capabilities

### Modified Capabilities

- `google-workspace-integration`: adds declarative action discovery, state facts, streaming `call` invocation, and patch proposal; removes integration-owned config writing and the in-binary setup wizard.

## Impact

- `src/descriptor.rs`: publish actions (id, role, title, inputs, requires).
- `src/state.rs` (new), `src/actions.rs` (new): state facts and action dispatch/streams.
- `src/setup.rs`: becomes the action layer; the guided menu and pasted-secret reader go away.
- `src/config.rs`: drop `merge_patch`, `backup_path`, `write_config_with_backup`, `summarize_patch`, `mcp_server_patch`; keep only reading/validation helpers.
- `src/main.rs`: new `state` and `call` subcommands.
- `Cargo.toml`: remove `inquire`.
- Tests, README, and `scripts/package-release.sh` smoke checks updated to the new commands.
- Supersedes the guided-menu requirements from the still-open `add-guided-google-workspace-setup` change; that change must be reconciled before archiving.
