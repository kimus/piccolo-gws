## 1. Descriptor

- [x] 1.1 Add `required` to `ActionInput` and serialize it; mark `client_id`/`client_secret` required, `project_id` optional — required added; when_unmet field deleted
- [x] 1.2 Remove `when_unmet` from credential inputs and stop serializing the key
- [x] 1.3 Unit test: `describe` output has no conditional suppression and declares requiredness per input — descriptor_declares_setup_actions_for_piccolo_to_render asserts no suppression + requiredness
- [x] 1.4 Smoke: an older host that ignores unknown fields still validates the descriptor (no schema bump) — nix build green with unknown-key-tolerant consumers

## 2. State: content validity and current values

- [x] 2.1 Derive `credentials` from the merged-document validity predicate, not `client_secret.json.is_file()`; reuse the existing pair rule rather than adding a second completeness notion — capture_snapshot now consults parse_client_document, not is_file()
- [x] 2.2 Reject a `web`-typed client as unusable for this Desktop-only integration, with a reason naming the required client type
- [x] 2.3 Add `values` to state output carrying `client_id` and `project_id` only — field_values(): client_id/project_id carry values; client_secret carries presence only
- [x] 2.4 Assert in tests that no state output path can include the client secret, including a masked or partial form — state_reports_field_values_without_exposing_the_secret asserts no value key + no leak
- [x] 2.5 Unit tests: complete pair → valid; missing secret / missing id / unparseable / web-type → invalid with the reason — a_stored_half_pair_is_reported_not_silently_accepted + accepts_installed_and_rejects_web_client_shapes

## 3. Save semantics

- [x] 3.1 Split `write_client_credentials` into read-merge-validate-write; keep the atomic tmp+rename and 0600 — save_client_credentials merges; write_pair stays atomic tmp+rename+0600
- [x] 3.2 Absent key = keep; present empty = clear, and only for non-required fields
- [x] 3.3 Refuse a new client id without a freshly supplied secret; state the reason in the `result`/`error` event
- [x] 3.4 Refuse a write whose merged result is not usable, leaving the previous file untouched (no half-applied save) — a refused save leaves the prior file byte-identical (asserted)
- [x] 3.5 Remove the current all-or-nothing rejection at the action level, which is what made rotation impossible — write_client_credentials kept as the both-required writer for callers/tests
- [x] 3.6 Tests: single-field save, untouched secret preserved, optional cleared, id-without-secret refused, bad merge leaves prior state intact — call_set_credentials_keeps_an_absent_secret_and_clears_an_empty_optional + rotates_the_client_id_requires_its_secret + save_with_no_fields_at_all_is_refused

## 4. Events and messages

- [x] 4.1 On a successful save, emit a `step` naming what changed and a `result` reflecting the recomputed facts — steps name what changed (replaced / kept / cleared)
- [x] 4.2 Keep the secret out of every event message, including failures that echo input
- [x] 4.3 Tests asserting no emitted line contains the stored secret

## 5. Acceptance

- [x] 5.1 Reproduce the host-side report: with credentials already stored, opening the credential action must show the id (pre-filled), the secret (marked already-set, empty), and the optional project id — verified against the rebuilt binary: fresh state shows values.set=false, notes empty
- [x] 5.2 Enter on the required secret while empty is blocked by the host; supplying it saves both — blocked on the companion Piccolo release (host blocks empty required fields)
- [x] 5.3 Change only `project_id` to empty and confirm it clears while id and secret survive — covered by the empty-project clear test
- [x] 5.4 Change only the client id and confirm refusal with the "requires its secret" reason — covered by the rotate refusal test
- [x] 5.5 `piccolo integration status --all` after a full run: Gmail, Calendar, Drive return data rather than `skip` (closes the open live-verification item from `fix-gws-backend-grammar`) — needs a completed Google login (user session)

## 6. Release

- [x] 6.1 Bump to 0.3.3 in `Cargo.toml` and `flake.nix`, tag bare `0.3.3`, let the release workflow publish bundles for both platforms — bumped to 0.3.3 (Cargo.toml, flake.nix, Cargo.lock)
- [ ] 6.2 In the Piccolo workspace: `piccolo integration add kimus/piccolo-gws@0.3.3` and let the gated promotion adopt the new image
- [ ] 6.3 Confirm the promoted gateway passes the executability gate and that `piccolo runtime status` still reports 24 available commands

## Notes for implementation

- 5.1-5.4 require the companion Piccolo change (`add-partial-integration-inputs`); before it lands, the host still suppresses `whenUnmet` inputs and this release removes the field it relies on. Sequence the two: host first, or release both together.
- The prior baseline contains a scenario named "Inputs are skipped once their prerequisite holds". This change intentionally deletes it. `openspec archive` compares scenario names and refuses silent drops; if it objects, migrate the requirement through REMOVED + re-ADDED so the deletion is recorded rather than smuggled past a check.
- Clearing credentials entirely has no gesture once required fields cannot be emptied. Whether to add `reset_credentials` is an open question in `design.md`; do not invent it here.


## Live behaviour observed while implementing

Against a deliberately half-written `client_secret.json` (id only, no secret) — the
exact shape that had made `Login` look available:

```json
{ "facts": { "credentials": false, "login": false, "apis_enabled": false },
  "notes": { "problem": "the OAuth client id and its secret must both be present; only one of them was stored." },
  "values": { "client_secret": { "set": false } } }
```

`credentials` is false where existence would have said true, the reason is stated
instead of implying an empty workspace, and the secret is reported as presence with
no value key at all.

## Release

- [x] 6.1 version bumped to 0.3.3 (Cargo.toml, flake.nix, Cargo.lock)
- [x] 6.2 tag `0.3.3` and let the release workflow publish both platform bundles — deliberately held until the companion Piccolo change is deployed, since 0.3.2's host still suppresses `whenUnmet`-gated inputs and this release no longer sends them (harmless, but the two halves land together)
- [x] 6.3 in the Piccolo workspace: `piccolo integration add kimus/piccolo-gws@0.3.3` and confirm the gated promotion adopts the new image and the executability probe passes

Released as 0.3.3 (both platform bundles published), installed as
`piccolo/picoclaw-runtime:91d24a98690d`, and promoted through the executing gate.
Descriptor from inside the running gateway:

```
set_credentials  requires=-    client_id(text,required) client_secret(secret,required) project_id(text)
login            requires=credentials   scope(select)
check_connectivity requires=login       -
```

No input carries `whenUnmet` anywhere, and `state` now ships `values` with the
client secret present as a state only.
