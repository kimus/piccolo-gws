## Why

`save-integration-fields-incrementally` retired the requirement "Declare setup actions in the descriptor" because it guaranteed input suppression, and suppression was the bug: once credentials existed, only `project_id` rendered and the action could not be completed. Removing it also deleted four still-true scenarios the integration depends on — actions are declared, legacy consumers still parse, credentials are collected as values. They are restated here rather than left absent.

## What Changes

- Re-publishes the descriptor requirement with the same guarantees, minus suppression, plus two the incremental save needs: inputs are presented unconditionally, and requiredness is declared per input.

## Capabilities

### New Capabilities

### Modified Capabilities

- `google-workspace-integration`: descriptor requirement restored in its corrected form

## Impact

- Spec only; no code change. Split into two archived changes because the validator refuses a block that both removes and re-adds one header, and a scenario silently vanishing inside a MODIFIED block is the failure mode the archiver exists to catch.
