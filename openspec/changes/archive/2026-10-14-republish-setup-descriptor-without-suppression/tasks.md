## 1. Restate the requirement

- [x] 1.1 Carry the four surviving scenarios from the retired block verbatim
- [x] 1.2 Drop "Inputs are skipped once their prerequisite holds"; add "Inputs are presented without state conditions"
- [x] 1.3 Add "Requiredness is declared per input", which shipped as 0.3.3

## 2. Confirm against shipped output

- [x] 2.1 The live descriptor from the promoted gateway declares `client_id`/`client_secret` as required and carries no `whenUnmet` anywhere
- [x] 2.2 Validate strict, then archive
