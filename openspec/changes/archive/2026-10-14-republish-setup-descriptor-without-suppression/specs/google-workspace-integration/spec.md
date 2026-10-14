## ADDED Requirements

### Requirement: Declare setup actions in the descriptor


The system SHALL publish its setup flow as a list of actions in `describe` output, each with a stable opaque id, a role of `setup` or `diagnostic`, a human-readable title, inputs that are always presented once the action is chosen, whether each input is required, and the state facts it requires, while keeping the descriptor schema identifier and the existing `setup.command` field populated for older hosts. Inputs SHALL NOT be conditioned on state facts: an input an action needs in order to do its job must be visible whenever that action is run.

#### Scenario: Actions are declared
- **WHEN** `piccolo-gws describe` is run
- **THEN** the output lists actions including credential setup, browser authorization, and connectivity diagnosis
- **AND** every input type is one of `text`, `secret`, `path`, `select`, `multiselect`, or `confirm`

#### Scenario: Legacy consumers still parse the descriptor
- **WHEN** a consumer that ignores unknown fields parses the descriptor
- **THEN** it still finds the existing schema identifier and a usable setup command
- **AND** the descriptor validates successfully without a schema bump

#### Scenario: Credentials are collected as values, not as a file
- **WHEN** the credential action is rendered
- **THEN** it asks for an OAuth client id (text) and an OAuth client secret (sensitive), matching the two values Google Cloud Console displays
- **AND** no action requires the user to download, locate, or copy a credentials file into the workspace

#### Scenario: Inputs are presented without state conditions
- **WHEN** credentials are already configured and the credential action is chosen again
- **THEN** the client id and client secret inputs are still declared, so they can be reviewed and replaced
- **AND** no input declares a state fact that would hide it

#### Scenario: Requiredness is declared per input
- **WHEN** the credential action is described
- **THEN** the client id and client secret inputs are marked required and the project id input is not
- **AND** requiredness is the only per-input condition expressed


## REMOVED Requirements
