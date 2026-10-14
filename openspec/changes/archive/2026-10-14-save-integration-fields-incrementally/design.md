## Context

Three code facts define this change.

```rust
non_empty("client_id") && non_empty("client_secret")   // setup.rs:91 — has_pair, the completeness rule
```
```rust
detect_credential_source(auth_dir.join("client_secret.json").is_file(), …)   // setup.rs:175 — the fact uses existence
```
```rust
/// Protocol emitter: stdout only, and never any secret value.   // actions.rs:184
```

The first two are equivalent **only because writes are all-or-nothing**: `write_client_credentials` refuses a partial pair (`setup.rs:116`), so every file on disk happens to be complete. Once a field may be saved alone, that coincidence becomes a bug — a file can exist and be unusable, and `credentials: true` would be a lie that unlocks `login`.

The third is why the host cannot pre-fill a secret. The user's requirement is that skip and clear be distinguishable, which for non-secret fields is satisfied by showing the stored value. For the secret, showing it means emitting it, so the distinction has to be carried by something else: the field renders empty, the host states that a value exists, and **absence means keep**.

## Goals / Non-Goals

**Goals:**
- Save one field without erroring about the fields not sent.
- Keep the pair rule as the definition of `credentials`, enforced by the same predicate that validates writes.
- Refuse a save that would leave a mismatched pair, and say why.
- Give the host the current values it may legitimately show.

**Non-Goals:**
- No emission of the client secret, not even masked or truncated.
- No new state or descriptor schema version.
- No clearing gesture for a *required* field. Required fields are rotated, never emptied; wiping credentials is a separate decision (see open questions).

## Decisions

### Absent means keep, empty means clear, and both are documented in the handler

`set_credentials` reads the stored document, overlays only the keys present in the answers, and validates the merged result. An absent `client_secret` therefore keeps the stored secret, which is what an untouched sensitive field means. An empty `project_id` clears that field, which is what a deliberately emptied optional field means. This is the behaviour the host now depends on, so the handler should carry it as a written rule rather than an emergent property of `unwrap_or("")`.

Rejected: requiring the full pair on every save (today's behaviour). That is the rule that made rotation impossible, and it cannot express "clear this optional value".

### `credentials` becomes content validity of a Desktop pair

Reuse `has_pair` rather than writing a second completeness notion, and reject `web`-typed documents: this integration authorizes as a Desktop app with an unregistered `http://localhost` redirect, so a `web` client is not merely different, it is unusable here and produces the `redirect_uri_mismatch` this project already spent time on. Existence keeps mattering for *detection* ("something is configured") but the fact that gates `login` must mean usable.

Consequence for hand-edited state: a malformed or partial `client_secret.json` on disk now yields `credentials: false` with a stated reason instead of silently unlocking login.

### An id change requires a fresh secret

Merging a new `client_id` onto a stored `client_secret` produces a document that satisfies `has_pair` and is still garbage: the pair rule sees presence, not correspondence. Only this integration can police it, so: if the submitted id differs from the stored id and no secret was submitted, reject with "changing the OAuth client also requires its secret". The alternative — accept and let Google fail — wastes a browser round trip on an error we can predict locally.

### Values go in state, not in the descriptor

`describe` is static capability; stored values change and are per-workspace. `state` already runs once per menu iteration on the host side, so adding `values` costs no extra invocation and keeps one read per loop. `client_id` and `project_id` are included; a `sensitive` input is never populated in `values`, so the host cannot accidentally render it — the shape of the data enforces the rule.

### `whenUnmet` is deleted here, not inverted

The host will ignore it either way, so leaving it in the descriptor would be a field that lies about behaviour. Deleting it makes the descriptor state what this version means: these inputs always appear, and `required` governs whether an empty one blocks submit.

## Risks / Trade-offs

- **Old hosts keep filtering.** A Piccolo predating this change still honours `whenUnmet`, which is now absent — so those hosts render all three fields too. Removing the field improves both directions; nothing depends on the old behaviour.
- **Merge makes writes non-atomic across fields** in the sense that one save no longer guarantees a complete pair. Mitigated by validating the merged result and by the id-change rule; the file is only ever written when the merge is coherent.
- **`credentials: false` after a bad hand-edit** may surprise a user whose file "looks present". The reason string has to name the defect, or this reads as the integration forgetting its own state.
- **Two-step rollout.** Until 0.3.3 is promoted, this workspace's bundle still declares `whenUnmet` and no `required`, so the host's new rules are partly inert.

## Open Questions

- Should there be a `reset_credentials` diagnostic/setup action, so removing credentials is an intentional verb? Under "required fields cannot be emptied" there is currently no gesture that ends with no client configured, which is a real gap for a shared machine.
- Should `check_connectivity` re-verify the pair against Google, giving the pair-correspondence problem a detection path beyond the local rule?
