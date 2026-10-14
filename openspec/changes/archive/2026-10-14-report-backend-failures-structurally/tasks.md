## 1. Parse the response, not the banner

- [x] 1.1 `parse_backend_failure` extracts code, reason, message and `enable_url`; falls back to stderr only with no envelope
- [x] 1.2 `extract_json_object` tolerates banners sharing stdout
- [x] 1.3 Structured fields ride through anyhow via a downcastable `ToolError` instead of being re-guessed from text
- [x] 1.4 `login_required` decided from status code and reason only

## 2. Say it usefully

- [x] 2.1 `check_connectivity` counts reachable services, names the unreachable ones, and passes the enable link through
- [x] 2.2 `status` carries `remediationLabel`/`remediationUrl`
- [x] 2.3 `login_required_for` no longer prints a container path a user cannot type
- [x] 2.4 `main.rs` emits an `internal` error event for any escaped error

## 3. Tests

- [x] 3.1 Real `accessNotConfigured` fixture: message is the provider's, banner absent, login_required false, URL present
- [x] 3.2 401 `unauthenticated` asserts login IS required — the opposite half of the same rule
- [x] 3.3 Prose containing "authorization" asserts no login prompt
- [x] 3.4 Banner-mixed stdout still parses
- [x] 3.5 Existing suite green (`nix build` runs cargo test for lib and integration targets)

## 4. Release and proof

- [x] 4.1 0.3.4 released for both platforms
- [x] 4.2 Installed into the workspace and promoted through the executability gate
- [x] 4.3 Live `piccolo integration status --all` against the still-disabled Drive API shows the real message and the clickable enable link
- [x] 4.4 Re-run after the user enabled Drive: `[OK] Drive: connected` and `gws: OK — Google Workspace connectivity checks passed`, so the message change did not mask a still-broken project
