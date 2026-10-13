## Context

Setup currently runs through raw stdin prompts. Real users hit confusing dead ends: the wizard suggests login without OAuth client credentials, the only credential path is manually placing a file inside the mounted state directory, and `gws auth setup` (which can create/select a GCP project + OAuth client) is unusable because we deliberately excluded Google Cloud SDK from the runtime bundle.

`gws` CLI (verified against `nixpkgs#gws`):

```text
gws auth login   Authenticate via OAuth2 (opens browser)
gws auth setup   Configure GCP project + OAuth client (requires gcloud)
gws auth status  Show current authentication state
gws auth logout  Clear saved credentials and token cache
```

`gws auth setup` is the only `gcloud` consumer, and it automates a step that happens once per workspace in Google Cloud Console. Measured closure sizes on `nixpkgs` unstable:

```text
nixpkgs#google-cloud-sdk   2.0 GiB
nixpkgs#gws               59.2 MiB
```

Release bundles carry their full Nix closure and Piccolo copies it into the runtime image, so bundling `gcloud` would put ~2 GiB into every release asset and every `integration add` download — the exact failure mode already reverted once in this repo.

Two distinct concepts must stay clear in the UI:

```text
OAuth client credentials  (client_secret.json / env vars)  → identifies the app
Google account login      (browser OAuth2 token cache)      → authorizes the user
```

Piccolo core provides a `huh`-based wizard in Go; `piccolo-gws` is Rust, so we use an equivalent Rust prompt library rather than coupling integrations to Piccolo UI internals.

## Goals / Non-Goals

**Goals:**
- Interactive `piccolo-gws setup` with an `inquire`-based guided menu.
- Normal path "Set up Google Workspace access" that imports credentials, then runs browser login, then patches PicoClaw config.
- Advanced actions: import credentials (paste `client_secret.json`), login/re-authorize, manual instructions, skip.
- Status-aware menu: explain missing prerequisites instead of failing late.
- Keep `--yes` fully non-interactive and idempotent.

**Non-Goals:**
- No automated Google Cloud project/OAuth provisioning (`gws auth setup` / `gcloud`); credential creation stays in Google Cloud Console.

**Non-Goals:**
- No Piccolo-core UI coupling (no generic Piccolo integration form protocol in this change).
- No write-capable Google scopes; tools stay read-only by default.
- No automatic Google Cloud Console navigation.
- No change to MCP server behavior or descriptor contract.

## Decisions

- **`inquire` for prompts** — closest Rust equivalent to `huh` (Select/Confirm/Text, terminal UX, well maintained). Alternative `dialoguer` has weaker list UX; alternative "render in Piccolo via a generic form protocol" couples Piccolo to every integration.
- **No `gcloud` anywhere** — automating credential creation costs a 2.0 GiB closure in every bundle and install, for a once-per-workspace action Google Cloud Console already covers interactively. "Create credentials" is therefore out of scope; import + manual instructions replace it. A future change could add a lazily-provisioned, setup-only image if console-driven setup proves painful.
- **Two entry modes** — top-level menu distinguishes the recommended guided flow from advanced one-purpose actions. Login is named "Login / authorize Google account" to stop conflating it with credential creation.
- **Import = paste, not file pick** — setup runs in a container; a host path is meaningless. Paste is validated with `serde_json` for a plausible client-secret shape (client id/secret present, installed/web structure recognized) and written to `<state-dir>/auth/client_secret.json`.
- **Menu built from a status snapshot** — `AuthSnapshot { gws, client, account }` computed once at start; each action declares its prerequisites, so unavailable actions explain the fix instead of crashing downstream.
- **Login runs with inherited stdio** — browser OAuth needs the printed URL/interaction to reach the user; after login, re-snapshot and report status.

## Risks / Trade-offs

- [Google Cloud SDK bloats bundles/closures] → Accept for now; track size in CI output and revisit with setup-only runtime image.
- [Browser OAuth inside container may not auto-open] → `gws auth login` prints URL; wizard tells the user to open it manually and warns port-forwarding may be needed for the redirect.
- [Manual credential creation is a UX cliff] → "Manual instructions" gives exact Console steps plus the precise container and host path to drop the file.
- [Pasted secrets are handled in-process] → never echo them back, write with best-effort restrictive perms, never log contents.
- [`gws auth setup` absent] → acceptable; users who want automated provisioning keep a host-side `gcloud` and create credentials there before importing.
- [`--yes` cannot do interactive auth] → keep skipping delegated auth with a hint, unchanged from today.

## Migration Plan

1. Add dependency, refactor setup into snapshot + menu + actions.
2. Drop `google-cloud-sdk` from the dev shell so nothing pulls it locally.
3. Release `piccolo-gws` patch version; users rerun `piccolo integration add` then `setup`.
4. Rollback: keep raw stdin fallback path for non-TTY environments.
