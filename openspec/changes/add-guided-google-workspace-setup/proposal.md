## Why

Google Workspace setup is currently too raw for a productized integration: users may be told to manually place `client_secret.json`, may hit confusing `gws auth login` failures when no OAuth client exists, and get no guided path from credentials to login. The integration should provide a first-run flow comparable to Piccolo's own setup UX.

## What Changes

- Add an interactive Rust setup wizard using `inquire` for Google Workspace auth/setup choices.
- Introduce a normal guided path: set up Google Workspace access by importing OAuth credentials, then authorizing the Google account in the browser.
- Add advanced setup actions: import credentials by pasting `client_secret.json`, login/re-authorize, manual instructions, and skip.
- Make setup status-aware so unavailable or invalid actions explain what prerequisite is missing instead of failing late.
- Preserve non-interactive `--yes` behavior and existing PicoClaw config patching semantics.

## Capabilities

### New Capabilities

### Modified Capabilities

- `google-workspace-integration`: Adds guided interactive setup requirements for OAuth credential import, browser login, and status-aware remediation.

## Impact

- `src/setup.rs`: Replace raw prompt flow with an `inquire`-based guided setup menu.
- `Cargo.toml`/lockfile: Add the Rust prompt dependency.
- `flake.nix`: drop the Google Cloud SDK from the dev shell; the runtime never depended on it.
- Tests: Cover menu decision helpers, credential import validation/storage, and non-interactive behavior where practical.
- README/docs: Document guided setup, the credentials-vs-login distinction, and expected browser OAuth behavior.

Credential *creation* stays manual (Google Cloud Console) rather than automated via `gws auth setup`: automating it requires the Google Cloud SDK, measured at a 2.0 GiB Nix closure versus 59 MiB for `gws`, which would be shipped in every release bundle and downloaded on every install for a once-per-workspace step.
