## Context

The guided `inquire` wizard shipped moments ago (tags `0.1.3`–`0.1.5`) is superseded by a design where Piccolo renders forms and integrations expose data. Verified facts driving this change:

```text
gws auth login        → prints the authorize URL on stderr, binds 127.0.0.1:<ephemeral>
                        inside the container, and blocks until the redirect arrives
client_secret.json    → carries installed.project_id, so console deep links are constructible
config.json writers   → piccolo-gws and Piccolo both patch it, with different backup names
```

`piccolo-gws` also already parses/writes everything needed: `serde_json`, the `AuthSnapshot` facts, and `backend_auth_state` for credential detection.

## Goals / Non-Goals

**Goals:**
- Describe actions and expose state so Piccolo can build the menu and gate it.
- One action entrypoint (`call <id>`) streaming structured events.
- Propose configuration changes instead of writing user-owned `config.json`.
- Accept host-staged files by container path so secrets never transit a terminal.
- Keep the non-interactive CLI usable for CI and direct container runs.

**Non-Goals:**
- No Piccolo changes here (the other side lives in `add-declarative-integrations`/`add-declarative-integration-setup`).
- No device-flow implementation: `gws` exposes no flag, so the `wait_redirect` courier is the reachable path.
- No new credentials-creation automation (`gcloud` stays out of the bundle).

## Decisions

- **`call <action-id>` rather than dynamic subcommands.** A dynamic id cannot be a clap subcommand without per-build dispatch; `call` is one static subcommand with an id argument, which also avoids collisions with `describe`/`state`.
- **Answers arrive as one JSON document on stdin.** Verified that argv is world-readable: `docker top` printed our own setup command line. Env is equally exposed via `docker inspect`. So secrets travel stdin → file only.
- **Three channels: stdout NDJSON, stderr human logs, stdin answers.** `gws` writes its authorize URL to stderr, leaving stdout clean for events — which is why events are ours to define and why no parsing of `gws` prose is needed.
- **`state` is JSON-only.** A mandatory `--json` flag is noise (already the case for `status`, which errors without it). New surface, so it starts without that wart.
- **Descriptor schema identifier stays unchanged.** `piccolo.integration.v1` is compared by equality in Piccolo, and unknown JSON fields are dropped, so adding `actions` is safe while a bump would hard-fail every existing Piccolo build. Consequence: every added field stays optional, and `piccolo-gws` must keep emitting the legacy `setup.command` so older Piccolo can still run a TTY-less fallback.
- **Actions are declared, credentials stay local.** `role` values are `setup`/`diagnostic`; ids are opaque to Piccolo. `state` keys are declared in the descriptor so Piccolo can validate that `requires` references something real.
- **Patch proposal replaces patching.** `result.configPatch` carries exactly the MCP server entry, mirroring today's `mcp_server_patch` shape; Piccolo validates against its allowlist and owns diff/confirm/`config.json.bak`/atomic write. Deleting the Rust equivalents removes the backup-name divergence.
- **`open_url` for console steps, with project-linked URLs.** Since `client_secret.json` includes `project_id`, we can emit `https://console.cloud.google.com/apis/credentials?project=<id>` rather than prose that says "go to the console".
- **`wait_redirect` for browser login.** The action prints the URL, emits the event, and serves its own loopback listener while Piccolo relays the captured request in. The action must therefore keep running and own the timeout, and must validate `state`/PKCE material it generated itself.
- **Keep a non-interactive path.** Direct `docker run … piccolo-gws call login` with an answers file remains supported for CI/headless, which is also how the courier is tested without a human.

## Risks / Trade-offs

- [Two UI generations live at once] → descriptor keeps both `setup.command` (legacy TTY, now minimal) and `actions`; Piccolo prefers actions. Reconcile `add-guided-google-workspace-setup` before archiving so its menu requirements don't resurface.
- [`inquire` removal changes the shipped binary's UX] → intended; the wizard moves to Piccolo. Document the new flow in README.
- [Browser callback still needs the host to relay] → acceptable for one machine; over SSH Piccolo can only print an `ssh -L` hint. File the `gws` device-flow request to remove this dependency.
- [Action timeouts] → the login action blocks on a human; it must bound its own wait and emit `error` rather than hanging forever.
- [Config patch shape drift] → keep one builder for the patch and assert its exact JSON in tests, since Piccolo now applies it verbatim.

## Migration Plan

1. Add `state`, then `call` with events; keep `setup` as a thin non-interactive wrapper during the transition.
2. Add `actions` to the descriptor while keeping `setup.command` populated.
3. Remove the `inquire` wizard and the config-writing helpers.
4. Release an integration version; verify through Piccolo `integration add` → `setup`.
5. Rollback: restore `setup.command`-driven TTY flow — Piccolo still honors it, so rollback is integration-only.
