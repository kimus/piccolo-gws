# piccolo-gws

Google Workspace integration package for Piccolo/PicoClaw.

`piccolo-gws` provides:

- a Piccolo integration descriptor via `piccolo-gws describe --json`
- a declarative setup menu Piccolo renders, with actions that prepare integration state and propose PicoClaw config changes
- a read-only stdio MCP server exposing curated Gmail, Calendar, and Drive tools
- a live connectivity status command for Google auth, Gmail, Calendar, and Drive

## Development environment

This project expects the Nix development shell for Rust tooling and runtime dependencies. Before using `cargo`, enter the shell:

```sh
nix develop
```

Then run cargo commands from inside that shell:

```sh
cargo fmt --check
cargo test
cargo build --release
cargo run -- describe --json
```

If you run `cargo` outside `nix develop`, the command may not be available or may miss expected dependencies.

## Build with Nix

```sh
nix build
./result/bin/piccolo-gws describe --json
```

The flake installs the `piccolo-gws` binary and wraps it with declared runtime dependencies on `PATH`. This remains the source-build/development path.

## Release bundles for Piccolo

Product Piccolo installs should use GitHub Release bundles instead of building this repository from source inside a user's runtime image. Release tags for this project are bare versions such as `3.1.0`.

Each Piccolo integration release uploads:

```text
piccolo-integration-release.json
piccolo-gws_<version>_linux_amd64.tar.gz
piccolo-gws_<version>_linux_arm64.tar.gz
checksums.txt
```

Each archive contains:

```text
bin/piccolo-gws
bin/gws
share/piccolo/integration/bundle.json
```

The bundle includes the lightweight `gws` backend. It does not include the Google Cloud SDK; see [Guided setup](#guided-setup).

## Commands

- `piccolo-gws describe --json` prints the Piccolo integration descriptor.
- `piccolo-gws state --config-dir <state-dir>` reports the facts Piccolo uses to gate its setup menu, as `piccolo.integration.state.v1` JSON.
- `piccolo-gws call <action-id> --config-dir <state-dir>` runs one action: answers arrive as JSON on **stdin**, structured events leave as NDJSON on **stdout**, human logs go to **stderr**.
- `piccolo-gws setup --config-dir <state-dir>` is a standalone, non-interactive report. It never writes `config.json`.
- `piccolo-gws mcp --config-dir <state-dir> --read-only` runs the stdio MCP server.
- `piccolo-gws status --config-dir <state-dir> --json` checks live Google Workspace connectivity and emits `piccolo.integration.status.v1` JSON for Piccolo.

## Setup

**Piccolo renders the setup UI.** This package declares an action menu in its
descriptor and executes one action per `call`; it ships no terminal wizard of
its own, so the experience matches `piccolo init`/`piccolo setup` for every
integration.

```sh
piccolo integration setup kimus/piccolo-gws
```

Actions are dispatched by opaque id, never by meaning:

| id | role | what it does |
| --- | --- | --- |
| `set_up_access` | setup | import credentials, then authorize the Google account |
| `import_credentials` | setup | adopt a `client_secret.json` Piccolo staged from the host |
| `login` | setup | browser OAuth2 against existing credentials |
| `check_connectivity` | diagnostic | probe Gmail, Calendar, and Drive through the mapped API methods |

Two different things are easy to confuse, so the menu keeps them separate:

| Concept | What it is | Provided by |
| --- | --- | --- |
| OAuth client credentials | identifies the *application* | `client_secret.json` created once in Google Cloud Console, then picked in Piccolo's file browser |
| Google account login | authorizes *your account* and stores refreshable tokens | the delegated backend's browser OAuth2 flow |

Credentials are needed once; login is needed once per account/scope set. An
action whose prerequisite fact is false is refused with
`prerequisite_unmet` and no side effects, so Piccolo never runs a doomed flow.
A leftover token without credentials is reported as unusable, not "configured".

### Secrets never touch a terminal or a command line

Answers arrive as one JSON document on stdin. Argv is world-readable through
`docker top` and environment through `docker inspect`, so neither carries a
credential. A `path` input is staged by Piccolo into this integration's state
directory with mode `0600`; the action receives the *container-side* path,
validates the client id/secret pair (top level, `installed`, or `web`), adopts
it, and deletes the staged duplicate. Secret values are never echoed into
events or logs, which the test suite asserts.

### Configuration is proposed, not written

This package never modifies `/root/.picoclaw/config.json`. A successful action
returns the MCP registration as `result.configPatch`, and Piccolo validates it
against a fixed allowlist (`tools.mcp.servers.google_workspace` only) before
showing a diff, asking for confirmation, and writing `config.json.bak`. One
writer means one backup convention and one confirmation gate for a file that
belongs to the user.

### Do not register a redirect URI

The login callback port is chosen by the operating system for every attempt, so it
differs each time and cannot be registered usefully:

```text
attempt 1 → http://localhost:37189
attempt 2 → http://localhost:34027
attempt 3 → http://localhost:36091
```

Create the OAuth client with application type **Desktop app**. That type needs no
registered redirect URI at all, because Google's installed-app flow accepts a
loopback redirect on any port. If Console shows an editable "Authorized redirect
URIs" list that you feel obliged to fill in, the client was created as a **Web
application**, which enforces an exact string match (including port) and can never
work here — `gws` also exposes no flag or environment variable to pin the port.

When `gws` reports `redirect_uri_mismatch`, this integration's `call login` action
turns it into an error event that says exactly that and links to the project's
credentials page (when a project id was supplied).

### Browser callbacks

The delegated backend binds an **ephemeral** loopback port inside the container
and waits for Google's redirect, which a published host port can never carry.
So the action emits `open_url` plus `wait_redirect`; Piccolo binds that port on
the host, and relays the browser's redirect back into the container's own
loopback. No ports are published by this integration.

Over SSH, print the URL and forward the port yourself
(`ssh -L <port>:localhost:<port> <host>`); Piccolo prints the hint. An upstream
`gws` device flow would remove this hop entirely — the CLI already links one
but exposes no flag for it.

The Google Cloud SDK (`gcloud`) is intentionally **not** a dependency of this
package. Its closure is about 2 GiB, would ship in every release bundle and
download on every install, and only automates a one-time step that Google Cloud
Console already covers interactively.

## MCP tools

The MCP server is read-only by default and exposes:

- `gmail_search`
- `gmail_read`
- `calendar_list`
- `calendar_read`
- `drive_search`
- `drive_read`

## Source layout

- `src/main.rs` - CLI parsing and command dispatch.
- `src/descriptor.rs` - Piccolo integration descriptor, including the action menu.
- `src/actions.rs` - action menu, state facts, and per-action execution with NDJSON events (credential import, browser authorization, connectivity).
- `src/setup.rs` - credential detection, `client_secret.json` validation/adoption, and the standalone non-interactive report.
- `src/mcp.rs` - stdio MCP server and tool handlers.
- `src/backend.rs` - wrapper around the delegated Google Workspace backend command.
- `src/config.rs` - state directory bootstrap and the MCP registration patch this integration proposes.
- `tests/cli.rs` - CLI integration tests.
- `docs/backend.md` and `docs/safety.md` - design notes.

## Safety

This first version is read-only. It exposes search/read tools for Gmail, Calendar, and Drive only. It does not expose send, create, update, delete, share, or arbitrary raw Google API operations.

Workspace content is untrusted input. Email, documents, calendar descriptions, and filenames may contain prompt-injection text. Treat returned content as data, not instructions.

## Piccolo setup containers

Piccolo discovers this package by running `piccolo-gws describe --json` in the gateway image, which returns the action menu. Setup then runs `piccolo-gws state` for gating facts and one `piccolo-gws call <action-id>` per user choice, with answers on stdin and events on stdout. Integration state lives under the package-scoped directory (`/root/.piccolo/integrations/<owner>/<repo>` in the container, `<workspace>/integrations/<owner>/<repo>` on the host). Piccolo, not this package, writes `/root/.picoclaw/config.json` after validating the proposed patch and showing a diff.

Piccolo can also run the described status command with `piccolo integration status kimus/piccolo-gws`. The status command performs live read-only checks against Google auth, Gmail, Calendar, and Drive by default, returning actionable remediation when authentication, scopes, APIs, or backend packaging need attention.

Authentication uses the delegated `gws` backend. `gws auth login` performs browser OAuth2 inside the Piccolo runtime image. OAuth client credentials are created once in Google Cloud Console and imported through setup (`Import credentials`) or supplied via `GOOGLE_WORKSPACE_CLI_CLIENT_ID` / `GOOGLE_WORKSPACE_CLI_CLIENT_SECRET`; release bundles deliberately exclude the multi-gigabyte Google Cloud SDK.

Expected state paths:

- `/root/.piccolo/integrations/gws/` - integration state root
- `/root/.piccolo/integrations/gws/auth/` - delegated Google auth/backend state
- `/root/.piccolo/integrations/gws/backend.json` - backend configuration/status metadata
- `/root/.picoclaw/config.json` - PicoClaw runtime config patched by setup
