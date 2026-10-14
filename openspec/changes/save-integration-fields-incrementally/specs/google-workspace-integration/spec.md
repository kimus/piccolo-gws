## MODIFIED Requirements

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

### Requirement: Report integration state facts

The system SHALL provide `piccolo-gws state` that prints, as JSON on stdout with no required flag, whether OAuth client credentials are usable, whether a Google account is authorized, and whether provider APIs are reachable, SHALL derive these facts from content validity rather than file existence, and SHALL also report the current value of each non-sensitive field so the host can present what is stored.

#### Scenario: Nothing configured
- **WHEN** neither client credentials nor an authorized account exist
- **THEN** state reports credentials and login as unmet

#### Scenario: Token present without credentials
- **WHEN** an authorization token exists but OAuth client credentials are missing
- **THEN** state reports credentials as unmet and login as not usable, rather than reporting a configured state

#### Scenario: Fully configured
- **WHEN** client credentials and an authorized account both exist
- **THEN** state reports both facts as met so the host can offer diagnosis actions

#### Scenario: A stored document that is not a usable pair
- **WHEN** a credentials file exists but lacks a client id, lacks a client secret, does not parse, or describes a client type this integration cannot use
- **THEN** state reports credentials as unmet
- **AND** the reason names what is missing rather than implying nothing is configured

#### Scenario: Current field values are reported without secrets
- **WHEN** state is printed
- **THEN** it includes the stored client id and project id so the host can pre-fill those fields
- **AND** it never includes the client secret, a masked stand-in for it, or any partial rendering of it

## ADDED Requirements

### Requirement: Save submitted fields and keep unsubmitted ones

The system SHALL apply each credential field it receives to its stored configuration, treating an absent key as leave-unchanged and an empty value for a non-required field as clear, and SHALL complete successfully when the merged result is usable even if only one field was submitted.

#### Scenario: One field saved at a time
- **WHEN** `call set_credentials` receives only a client id and a stored secret already exists
- **THEN** the merged configuration keeps the stored secret and reports the save as successful
- **AND** state recomputes from the merged result

#### Scenario: An optional value is cleared
- **WHEN** the answers carry an empty project id
- **THEN** the stored project id is unset
- **AND** no other field is altered

#### Scenario: An untouched secret is never wiped
- **WHEN** the answers contain no client secret key
- **THEN** the stored client secret is preserved rather than cleared

#### Scenario: The merged result is validated before writing
- **WHEN** the merge would produce a document that is not a usable pair
- **THEN** the write is refused and the existing stored configuration remains intact

### Requirement: A changed client id requires its secret

The system SHALL refuse to store a new OAuth client id together with a previously stored client secret, since presence of both values does not mean they belong to the same client.

#### Scenario: Id rotated alone
- **WHEN** the submitted client id differs from the stored one and no client secret was submitted
- **THEN** the save is refused
- **AND** the reason states that changing the OAuth client also requires entering its secret

#### Scenario: Id and secret rotated together
- **WHEN** a new client id arrives with a newly typed secret
- **THEN** both are stored as a pair

#### Scenario: Same id with a new secret
- **WHEN** the submitted client id equals the stored one and a new secret is supplied
- **THEN** the secret alone is replaced

### Requirement: Desktop client type is part of validity

The system SHALL treat a credentials document describing a non-Desktop client as not usable, because this integration authorizes as a Desktop application whose redirect target is not registered in the provider console.

#### Scenario: Web-type client document stored
- **WHEN** a credentials document carries a `web` client object rather than an `installed` one
- **THEN** validity reports the credentials as unusable
- **AND** the reported reason explains that a Desktop OAuth client is required, rather than allowing an authorization attempt that the provider will reject

### Requirement: Retire conditional input suppression

The system SHALL stop publishing state-conditioned input suppression, so that no consumer can omit a field the credential action depends on.

#### Scenario: Descriptor after retirement
- **WHEN** the descriptor is published by this version
- **THEN** no input carries a fact that would hide it
- **AND** hosts that never implemented suppression and hosts that ignore the field both render the same complete form
