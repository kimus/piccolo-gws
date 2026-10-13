# Tasks: fix-gws-backend-grammar

## 1. Correct the backend invocation mapping

- [x] 1.1 Replace the `gmail_search` argv builder with `gmail users messages list --params {userId,q,maxResults}`
- [x] 1.2 Replace `gmail_read` with `gmail users messages get --params {userId,id,format}`
- [x] 1.3 Replace `calendar_list` with `calendar events list --params {calendarId,timeMin,timeMax,maxResults}`
- [x] 1.4 Replace `calendar_read` with `calendar events get --params {calendarId,eventId}`
- [x] 1.5 Replace `drive_search` with `drive files list --params {q,pageSize}`
- [x] 1.6 Replace `drive_read` with `drive files get --params {fileId}`
- [x] 1.7 Build `--params` values with `serde_json` instead of string flags, keeping argv free of shell quoting

## 2. Defaults and tool surface

- [x] 2.1 Default `userId` to `me` and `calendarId` to `primary`, with optional overrides when the tool args provide them
- [x] 2.2 Keep tool names, read-only behavior, and the login-required response shape unchanged
- [x] 2.3 Update `src/mcp.rs` tool schemas/docs where parameter names or defaults changed

## 3. Status probes

- [x] 3.1 Confirm status probes route through the corrected tool mapping and no longer emit `unrecognized subcommand`
- [x] 3.2 Adjust probe arguments so a failing check means auth/scope/API trouble, not grammar

## 4. Verification

- [x] 4.1 Add unit tests asserting the exact argv (service/resource/method + `--params` JSON) produced for each of the six tools
- [x] 4.2 Add a packaging-time grammar check using `gws schema <method>` for every mapped method
- [x] 4.3 Run `nix develop -c cargo fmt --check`, `nix develop -c cargo clippy --all-targets -- -D warnings`, `nix develop -c cargo test`, `nix build .#`
- [ ] 4.4 Verify live against a real authenticated workspace that Gmail, Calendar, and Drive reads return data
