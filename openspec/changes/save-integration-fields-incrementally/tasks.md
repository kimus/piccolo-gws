## 1. Descriptor

- [ ] 1.1 Add `required` to `ActionInput` and serialize it; mark `client_id`/`client_secret` required, `project_id` optional
- [ ] 1.2 Remove `when_unmet` from credential inputs and stop serializing the key
- [ ] 1.3 Unit test: `describe` output has no conditional suppression and declares requiredness per input
- [ ] 1.4 Smoke: an older host that ignores unknown fields still validates the descriptor (no schema bump)

## 2. State: content validity and current values

- [ ] 2.1 Derive `credentials` from the merged-document validity predicate, not `client_secret.json.is_file()`; reuse the existing pair rule rather than adding a second completeness notion
- [ ] 2.2 Reject a `web`-typed client as unusable for this Desktop-only integration, with a reason naming the required client type
- [ ] 2.3 Add `values` to state output carrying `client_id` and `project_id` only
- [ ] 2.4 Assert in tests that no state output path can include the client secret, including a masked or partial form
- [ ] 2.5 Unit tests: complete pair → valid; missing secret / missing id / unparseable / web-type → invalid with the reason

## 3. Save semantics

- [ ] 3.1 Split `write_client_credentials` into read-merge-validate-write; keep the atomic tmp+rename and 0600
- [ ] 3.2 Absent key = keep; present empty = clear, and only for non-required fields
- [ ] 3.3 Refuse a new client id without a freshly supplied secret; state the reason in the `result`/`error` event
- [ ] 3.4 Refuse a write whose merged result is not usable, leaving the previous file untouched (no half-applied save)
- [ ] 3.5 Remove the current all-or-nothing rejection at the action level, which is what made rotation impossible
- [ ] 3.6 Tests: single-field save, untouched secret preserved, optional cleared, id-without-secret refused, bad merge leaves prior state intact

## 4. Events and messages

- [ ] 4.1 On a successful save, emit a `step` naming what changed and a `result` reflecting the recomputed facts
- [ ] 4.2 Keep the secret out of every event message, including failures that echo input
- [ ] 4.3 Tests asserting no emitted line contains the stored secret

## 5. Acceptance

- [ ] 5.1 Reproduce the host-side report: with credentials already stored, opening the credential action must show the id (pre-filled), the secret (marked already-set, empty), and the optional project id
- [ ] 5.2 Enter on the required secret while empty is blocked by the host; supplying it saves both
- [ ] 5.3 Change only `project_id` to empty and confirm it clears while id and secret survive
- [ ] 5.4 Change only the client id and confirm refusal with the "requires its secret" reason
- [ ] 5.5 `piccolo integration status --all` after a full run: Gmail, Calendar, Drive return data rather than `skip` (closes the open live-verification item from `fix-gws-backend-grammar`)

## 6. Release

- [ ] 6.1 Bump to 0.3.3 in `Cargo.toml` and `flake.nix`, tag bare `0.3.3`, let the release workflow publish bundles for both platforms
- [ ] 6.2 In the Piccolo workspace: `piccolo integration add kimus/piccolo-gws@0.3.3` and let the gated promotion adopt the new image
- [ ] 6.3 Confirm the promoted gateway passes the executability gate and that `piccolo runtime status` still reports 24 available commands

## Notes for implementation

- 5.1-5.4 require the companion Piccolo change (`add-partial-integration-inputs`); before it lands, the host still suppresses `whenUnmet` inputs and this release removes the field it relies on. Sequence the two: host first, or release both together.
- The prior baseline contains a scenario named "Inputs are skipped once their prerequisite holds". This change intentionally deletes it. `openspec archive` compares scenario names and refuses silent drops; if it objects, migrate the requirement through REMOVED + re-ADDED so the deletion is recorded rather than smuggled past a check.
- Clearing credentials entirely has no gesture once required fields cannot be emptied. Whether to add `reset_credentials` is an open question in `design.md`; do not invent it here.
