## Context

Setup currently runs through raw stdin prompts. Real users hit confusing dead ends: the wizard suggests login without OAuth client credentials, the only credential path is manually placing a file inside the mounted state directory, and `gws auth setup` (which can create/select a GCP project + OAuth client) is unusable because we deliberately excluded Google Cloud SDK from the runtime bundle.

`gws` CLI (verified against `nixpkgs#gws`):

```text
gws auth login   Authenticate via OAuth2 (opens browser)
gws auth setup   Configure GCP project + OAuth client (requires gcloud)
                 --project <id>  Use a specific GCP project
                 --login         Run `gws auth login` after successful setup
                 --dry-run       Preview changes without making them
gws auth status  Show current authentication state
gws auth logout  Clear saved credentials and token cache
```

Two distinct concepts must stay clear in the UI:

```text
OAuth client credentials  (client_secret.json / env vars)  → identifies the app
Google account login      (browser OAuth2 token cache)      → authorizes the user
```

Piccolo core provides a `huh`-based wizard in Go; `piccolo-gws` is Rust, so we use an equivalent Rust prompt library rather than coupling integrations to Piccolo UI internals.

## Goals / Non-Goals

**Goals:**
- Interactive `piccolo-gws setup` with an `inquire`-based guided menu.
- Normal path "Set up Google Workspace access" that creates or imports credentials, then runs browser login, then patches PicoClaw config.
- Advanced actions: create credentials only, import credentials (paste `client_secret.json`), login/re-authorize, manual instructions, skip.
- Status-aware menu: explain missing prerequisites instead of failing late.
- `gcloud` available again in the runtime so `gws auth setup` works end to end.
- Keep `--yes` fully non-interactive and idempotent.

**Non-Goals:**
- No Piccolo-core UI coupling (no generic Piccolo integration form protocol in this change).
- No write-capable Google scopes; tools stay read-only by default.
- No automatic Google Cloud Console navigation beyond what `gws auth setup`/`gcloud` already do.
- No change to MCP server behavior or descriptor contract.

## Decisions

- **`inquire` for prompts** — closest Rust equivalent to `huh` (Select/Confirm/Text, terminal UX, well maintained). Alternative `dialoguer` has weaker list UX; alternative "render in Piccolo via a generic form protocol" couples Piccolo to every integration.
- **Re-add `google-cloud-sdk` to the runtime bundle** — "Create credentials" requires `gcloud`; UX wins over bundle size (user decision). Size regression to be revisited later via a setup-only image.
- **Two entry modes** — top-level menu distinguishes the recommended guided flow from advanced one-purpose actions. Login is named "Login / authorize Google account" to stop conflating it with credential creation.
- **Import = paste, not file pick** — setup runs in a container; a host path is meaningless. Paste is validated with `serde_json` for a plausible client-secret shape (client id/secret present, installed/web structure recognized) and written to `<state-dir>/auth/client_secret.json`.
- **Menu built from a status snapshot** — `AuthSnapshot { gws, gcloud, client, tokens }` computed once at start; each action declares its prerequisites, so unavailable actions explain the fix instead of crashing downstream.
- **Login runs with inherited stdio** — browser OAuth needs the printed URL/interaction to reach the user; after login, re-snapshot and report status.

## Risks / Trade-offs

- [Google Cloud SDK bloats bundles/closures] → Accept for now; track size in CI output and revisit with setup-only runtime image.
- [Browser OAuth inside container may not auto-open] → `gws auth login` prints URL; wizard tells the user to open it manually and warns port-forwarding may be needed for the redirect.
- [Pasted secrets are handled in-process] → never echo them back, write with best-effort restrictive perms, never log contents.
- [`gws auth setup` UX owned by upstream] → surface its output directly and fall back to manual instructions on non-zero exit.
- [`--yes` cannot do interactive auth] → keep skipping delegated auth with a hint, unchanged from today.

## Migration Plan

1. Add dependency, refactor setup into snapshot + menu + actions.
2. Add `gcloud` back to flake packages and release bundle.
3. Release `piccolo-gws` patch version; users rerun `piccolo integration add` then `setup`.
4. Rollback: keep raw stdin fallback path for non-TTY environments.
