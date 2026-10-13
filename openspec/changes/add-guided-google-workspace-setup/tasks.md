# Tasks: add-guided-google-workspace-setup

## 1. Dependencies and auth snapshot

- [x] 1.1 Add `inquire` (and `serde_json` if not already present) to `Cargo.toml` and update `Cargo.lock`
- [x] 1.2 Introduce an `AuthSnapshot` capturing: `gws` present, `gcloud` present, OAuth client configured (file or env), account token/login state from `gws auth status`
- [x] 1.3 Replace the raw `has_oauth_client`/`gws_auth_status` helpers with snapshot-backed checks and unit-test credential detection (file, env pair, neither)

## 2. Guided setup menu

- [x] 2.1 Render an `inquire` select menu with "Set up Google Workspace access" plus advanced actions: Create credentials, Import credentials, Login / authorize Google account, Manual instructions, Skip
- [x] 2.2 Print a human-readable auth state summary before the menu, avoiding "configured" when only a token exists without credentials
- [x] 2.3 Gate interactive setup on TTY availability and keep `--yes` fully non-interactive (no prompt rendering)

## 3. Guided access flow

- [x] 3.1 Implement ordered guided flow: ensure credentials, then authorize account, then PicoClaw config preview/patch
- [x] 3.2 When credentials are missing and `gcloud` is available, offer credential creation via `gws auth setup` and surface its output
- [x] 3.3 When credentials are missing and `gcloud` is unavailable, explain the Google Cloud SDK prerequisite and offer import or manual instructions instead of running a doomed command
- [x] 3.4 When credentials already exist, skip creation and proceed to authorization
- [x] 3.5 Run browser login via `gws auth login` with inherited stdio, then re-snapshot and report status
- [x] 3.6 After credential creation or login failure/decline, loop back to offer import or manual instructions

## 4. Advanced actions

- [x] 4.1 Implement "Create credentials" as a standalone action reusing the `gws auth setup` path
- [x] 4.2 Implement "Import credentials" accepting pasted `client_secret.json`, validating non-empty client id and client secret, and writing `<state-dir>/auth/client_secret.json` without echoing the secret
- [x] 4.3 Reject invalid pasted content with an explanation and write nothing
- [x] 4.4 Require explicit confirmation before overwriting an existing `client_secret.json`
- [x] 4.5 Implement "Login / authorize Google account" as a standalone action that refuses to run `gws auth login` when no credentials exist and routes the user to create/import instead
- [x] 4.6 Implement "Manual instructions" printing Google Cloud Console steps plus container-side and host-side `client_secret.json` paths and the environment-variable alternative
- [x] 4.7 Implement "Skip" continuing setup without authentication and stating tools will report login-required

## 5. Google Cloud SDK availability

- [ ] 5.1 Add `google-cloud-sdk` back to the runtime dependency set in `flake.nix` so `gcloud` is on PATH for setup
- [ ] 5.2 Update `scripts/package-release.sh` to include `gcloud` in the bundle closure and remove the assertion that the bundle must not contain `gcloud`
- [ ] 5.3 Verify a packaged bundle exposes both `gws auth login` and `gcloud --version`, and record resulting bundle size

## 6. Verification

- [x] 6.1 Update/extend tests in `tests/cli.rs` and unit tests for: credential detection, import validation (valid/invalid/overwrite confirm), login gating without credentials, and `--yes` non-interactive config patch
- [x] 6.2 Run `nix develop -c cargo fmt --check`, `nix develop -c cargo test`, `nix build .#`
- [ ] 6.3 Exercise interactive setup against a real workspace and confirm: guided create-credentials → browser login → config patch, and import → login → config patch
- [ ] 6.4 Update `README.md` documenting guided setup, the credentials-vs-login distinction, `gcloud` availability, and browser OAuth/port-forwarding caveat
