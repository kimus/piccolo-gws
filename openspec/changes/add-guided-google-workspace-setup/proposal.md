## Why

Google Workspace setup is currently too raw for a productized integration: users may be told to manually place `client_secret.json`, may hit confusing `gws auth login` failures when no OAuth client exists, and cannot create credentials from the guided setup path. The integration should provide a first-run flow comparable to Piccolo's own setup UX.

## What Changes

- Add an interactive Rust setup wizard using `inquire` for Google Workspace auth/setup choices.
- Re-add Google Cloud SDK availability to the integration runtime so `gws auth setup` can create/select a Google Cloud project and OAuth credentials.
- Introduce a normal guided path: set up Google Workspace access by creating/importing OAuth credentials, then authorizing the Google account in the browser.
- Add advanced setup actions: create credentials only, import credentials by pasting `client_secret.json`, login/re-authorize, manual instructions, and skip.
- Make setup status-aware so unavailable or invalid actions explain what prerequisite is missing instead of failing late.
- Preserve non-interactive `--yes` behavior and existing PicoClaw config patching semantics.

## Capabilities

### New Capabilities

### Modified Capabilities

- `google-workspace-integration`: Adds guided interactive setup requirements for OAuth credential creation/import, browser login, Google Cloud SDK support, and status-aware remediation.

## Impact

- `src/setup.rs`: Replace raw prompt flow with an `inquire`-based guided setup menu.
- `Cargo.toml`/lockfile: Add the Rust prompt dependency.
- `flake.nix` and release packaging: Make `gcloud` available in the runtime bundle again for setup-time credential creation.
- Tests: Cover menu decision helpers, credential import validation/storage, and non-interactive behavior where practical.
- README/docs: Document guided setup, credential/login distinction, and expected browser OAuth behavior.

This change supersedes the "bundles must not contain `gcloud`" decision in the still-open
`publish-integration-release-artifacts` change; that change's artifacts were updated to match.
