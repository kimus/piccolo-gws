## MODIFIED Requirements

### Requirement: Run integration setup

The system SHALL provide `piccolo-gws setup` as a non-interactive setup entry point that configures Google Workspace integration state from inside a Piccolo setup container, and SHALL NOT write PicoClaw runtime configuration itself.

#### Scenario: Setup uses config directory
- **WHEN** setup is run without an explicit config directory
- **THEN** Google Workspace integration state is read from and written under `/root/.piccolo/integrations/gws`

#### Scenario: Setup proposes rather than writes runtime config
- **WHEN** a setup action completes successfully and the MCP server entry is not yet registered
- **THEN** the action result carries a configuration patch registering `piccolo-gws mcp --config-dir <state-dir> --read-only`
- **AND** the system does not modify `/root/.picoclaw/config.json`

#### Scenario: Runtime config remains user-owned
- **WHEN** any setup or action path runs
- **THEN** the system never creates, replaces, or backs up `config.json`
- **AND** unknown keys and unrelated sections are preserved by construction, because the file is not touched

## ADDED Requirements

### Requirement: Declare setup actions in the descriptor

The system SHALL publish its setup flow as a list of actions in `describe` output, each with a stable opaque id, a role of `setup` or `diagnostic`, a human-readable title, optional typed inputs, and the state facts it requires, while keeping the descriptor schema identifier and the existing `setup.command` field populated for older hosts.

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

#### Scenario: Inputs are skipped once their prerequisite holds
- **WHEN** an input declares a `whenUnmet` fact that the current state reports as satisfied
- **THEN** the system expects that input to be omitted from the form

### Requirement: Report integration state facts

The system SHALL provide `piccolo-gws state` that prints, as JSON on stdout with no required flag, whether OAuth client credentials exist, whether a Google account is authorized, and whether provider APIs are reachable, and SHALL derive these facts from the backend's reported state rather than process exit codes.

#### Scenario: Nothing configured
- **WHEN** neither client credentials nor an authorized account exist
- **THEN** state reports credentials and login as unmet

#### Scenario: Token present without credentials
- **WHEN** an authorization token exists but OAuth client credentials are missing
- **THEN** state reports credentials as unmet and login as not usable, rather than reporting a configured state

#### Scenario: Fully configured
- **WHEN** client credentials and an authorized account both exist
- **THEN** state reports both facts as met so the host can offer diagnosis actions

### Requirement: Invoke actions by id with structured streaming

The system SHALL provide `piccolo-gws call <action-id>`, reading collected answers as a single JSON document from stdin and emitting newline-delimited JSON events on stdout while human-readable logs go to stderr.

#### Scenario: Answers are read from stdin
- **WHEN** an action is invoked with an answers document on stdin
- **THEN** the system consumes the answers without accepting any answer value as a command argument or environment variable

#### Scenario: Progress is streamed
- **WHEN** an action performs multiple steps
- **THEN** the system emits one event per step with its status and message as the step happens, before the process exits

#### Scenario: Unknown action id
- **WHEN** `call` is invoked with an id the system does not implement
- **THEN** the system emits an error event naming the unknown id and exits non-zero

#### Scenario: Required prerequisite is missing
- **WHEN** an action is invoked while its required state facts are unmet
- **THEN** the system emits an error event with remediation text and performs no side effects

#### Scenario: Secrets are never echoed
- **WHEN** an action handles a client secret or authorization credential
- **THEN** no event, log line, or error message contains the secret value

### Requirement: Surface console steps as links

The system SHALL emit an `open_url` event, with a short label and a URL, for each user step that must happen in Google Cloud Console, constructing project-linked URLs from the installed client configuration when a project id is available.

#### Scenario: Credentials are missing
- **WHEN** the credential action is offered and no client credentials exist
- **THEN** the system emits an `open_url` event pointing at the Google Cloud Console credentials page, with a one-line label describing what to create

#### Scenario: Project is known
- **WHEN** stored client configuration includes a project id
- **THEN** the emitted URL is deep-linked to that project rather than the console root

### Requirement: Await browser callbacks through a host courier

The system SHALL emit a `wait_redirect` event carrying the authorization URL for browser login, keep its own loopback listener running in the container, and complete the exchange using the raw request line the host relays back, bounding the wait with its own timeout.

#### Scenario: Authorization URL is published
- **WHEN** browser login starts
- **THEN** the system emits `wait_redirect` with the full authorization URL including its loopback redirect target, and the URL is not written to stdout as plain prose

#### Scenario: Relayed callback completes login
- **WHEN** the host relays the captured request path and query into the container's loopback listener
- **THEN** the system exchanges the code, stores credentials under the integration state directory, and emits a success result event

#### Scenario: Callback does not arrive
- **WHEN** no relayed request arrives before the action's wait window ends
- **THEN** the system emits an error event stating that authorization did not complete, closes its listener, and exits non-zero without touching runtime configuration

### Requirement: Keep a usable non-interactive path

The system SHALL remain runnable without a terminal so actions can be driven from scripts and tests by supplying an answers document on stdin.

#### Scenario: Action runs without a TTY
- **WHEN** `call` runs with stdin redirected from a file and no terminal attached
- **THEN** the system executes the action, emits only machine events on stdout, and exits according to the result

#### Scenario: Host-driven test of the courier
- **WHEN** a test invokes the login action and relays a callback request without any interactive terminal
- **THEN** the action completes through the same relayed-callback path a real host would use
