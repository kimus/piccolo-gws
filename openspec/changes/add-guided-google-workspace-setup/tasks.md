# Tasks: add-guided-google-workspace-setup

## 1. Dependencies and auth snapshot

- [x] 1.1 Add `inquire` (and `serde_json` if not already present) to `Cargo.toml` and update `Cargo.lock`
- [x] 1.2 Introduce an `AuthSnapshot` capturing: `gws` present, OAuth client configured (file or env), account authorization state from `gws auth status`
- [x] 1.3 Replace the raw `has_oauth_client`/`gws_auth_status` helpers with snapshot-backed checks and unit-test credential detection (file, env pair, neither)

## 2. Guided setup menu

- [x] 2.1 Render an `inquire` select menu with "Set up Google Workspace access" plus advanced actions: Import credentials, Login / authorize Google account, Manual instructions, Skip
- [x] 2.2 Print a human-readable auth state summary before the menu, avoiding "configured" when only a token exists without credentials
- [x] 2.3 Gate interactive setup on TTY availability and keep `--yes` fully non-interactive (no prompt rendering)

## 3. Guided access flow

- [x] 3.1 Implement ordered guided flow: ensure credentials, then authorize account, then PicoClaw config preview/patch
- [x] 3.2 When credentials are missing, explain that they identify the application and are created in Google Cloud Console, then offer import or manual instructions
- [x] 3.3 When credentials already exist, skip credential handling and proceed to authorization
- [x] 3.4 Run browser login via `gws auth login` with inherited stdio, then re-snapshot and report status
- [x] 3.5 After an import decline or login failure, loop back so the user can retry, view instructions, or skip

## 4. Advanced actions

- [x] 4.1 Implement "Import credentials" accepting pasted `client_secret.json`, validating a non-empty client id and client secret (top level, `installed`, or `web`), and writing `<state-dir>/auth/client_secret.json` without echoing the secret
- [x] 4.2 Reject invalid pasted content with an explanation and write nothing
- [x] 4.3 Require explicit confirmation before overwriting an existing `client_secret.json`
- [x] 4.4 Implement "Login / authorize Google account" as a standalone action that refuses to run `gws auth login` when no credentials exist and routes the user to import or manual instructions
- [x] 4.5 Implement "Manual instructions" printing Google Cloud Console steps plus container-side and host-side `client_secret.json` paths and the environment-variable alternative
- [x] 4.6 Implement "Skip" continuing setup without authentication and stating tools will report login-required

## 5. No Google Cloud SDK

- [x] 5.1 Confirm no setup, status, or runtime path invokes `gcloud`/`gws auth setup`
- [x] 5.2 Drop `google-cloud-sdk` from the `flake.nix` dev shell and keep the runtime dependency set at `gws` only (measured: SDK closure 2.0 GiB vs `gws` 59 MiB)
- [x] 5.3 Keep the release packaging assertion that bundles must not contain `gcloud`

## 6. Verification

- [x] 6.1 Update/extend tests for: credential detection, import validation (valid/invalid/overwrite confirm), login gating without credentials, host-path translation, and `--yes` non-interactive config patch
- [x] 6.2 Run `nix develop -c cargo fmt --check`, `nix develop -c cargo clippy --all-targets -- -D warnings`, `nix develop -c cargo test`, `nix build .#`
- [x] 6.3 Update `README.md` documenting guided setup, the credentials-vs-login distinction, the deliberate absence of `gcloud`, and the browser OAuth/port-forwarding caveat
- [ ] 6.4 Exercise interactive setup against a real workspace and confirm: import → browser login → config patch, plus the gated path when credentials are missing
