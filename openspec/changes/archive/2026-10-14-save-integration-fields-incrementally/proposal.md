## Why

The host that renders this integration's setup form deleted two of its three fields. `client_id` and `client_secret` are declared with `whenUnmet: credentials`, so once credentials existed the credential action could only ever ask for `project_id` — and submitting that one field produced `missing_input`. The action became unfillable for the single reason it exists: it had already been filled once. Rotation of a client secret has no path.

The host-side change (never omit a declared input, pre-fill from state, block empty submits) needs a partner here, because this integration is the one that knows the current values and the one that must treat "field absent" as *keep* rather than as *clear*.

## What Changes

- Every credential input is declared, and **`required`** replaces `whenUnmet` as the property of an input: `client_id` and `client_secret` are required, `project_id` is optional. `whenUnmet` is dropped from the descriptor.
- `state` gains a **`values`** object so the host can show what is stored — `client_id` and `project_id` only. The client secret is never emitted, in any form, including a mask standing in for it.
- `set_credentials` becomes a **save**, not an all-or-nothing write: it merges what it receives onto what it stores, treating an absent key as keep and an empty value as clear, and it recomputes `credentials` from the merged result.
- The `credentials` fact becomes **content validity of a Desktop pair**, not file existence: a document that parses, carries a non-empty id and secret together, and is `installed`-typed. A `web`-typed client — the shape that produced the earlier `redirect_uri_mismatch` — no longer counts as configured-and-usable.
- An id change without a fresh secret is rejected with a stated reason rather than merged into a mismatched pair.

## Capabilities

### New Capabilities

### Modified Capabilities

- `google-workspace-integration`: the descriptor's inputs are never conditional, state also reports current field values, and credential saving is per-field merge with content-derived validity

## Impact

- `src/actions.rs` — `credential_inputs` gains `required`, loses `when_unmet`; `set_credentials` becomes merge-based; `state_facts` derives `credentials` from a validity check and also emits values
- `src/setup.rs` — `write_client_credentials` splits into read-merge-validate-write; `validate_client_secret` rejects `web`-typed documents for this Desktop-only integration; add a helper that reports which half of the pair is present
- `src/descriptor.rs` — input serialization for `required`; drop `whenUnmet` output
- `tests/cli.rs` — save-one-field, keep-untouched-secret, clear-optional, id-changed-without-secret, web-client-rejected
- Release: this is the 0.3.3 side. Workspaces on 0.3.2 keep the old descriptor until `piccolo integration add` + promote
