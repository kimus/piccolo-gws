# google-workspace-integration Specification

## Purpose

Define the Google Workspace integration package for Piccolo, including installation metadata, setup, authentication/configuration behavior, read-only MCP tools, and safety defaults.
## Requirements
### Requirement: Provide self-describing integration binary
The system SHALL provide a `piccolo-gws` executable with a `describe --json` command that returns a supported Piccolo integration descriptor using Piccolo-owned integration state paths, including setup, MCP, and status command metadata.

#### Scenario: Descriptor command succeeds
- **WHEN** `piccolo-gws describe --json` is run
- **THEN** it prints JSON with schema `piccolo.integration.v1`, integration name `gws`, display name `Google Workspace`, setup command metadata, MCP command metadata, and status command metadata

#### Scenario: Descriptor uses Piccolo integration state path
- **WHEN** Piccolo reads the descriptor
- **THEN** the descriptor setup, MCP, and status command metadata use `/root/.piccolo/integrations/gws` as the default `--config-dir`

### Requirement: Provide installable Nix flake package
The system SHALL be installable as a Nix flake package suitable for installation into a Piccolo gateway image.

#### Scenario: Package is installed
- **WHEN** the package reference `github:kimus/piccolo-gws` is installed during a Docker build
- **THEN** the `piccolo-gws` command is available on PATH in the resulting image

#### Scenario: Runtime dependencies are included
- **WHEN** `piccolo-gws` requires a backend such as `gws`
- **THEN** the package or wrapper provides the runtime dependency without requiring Piccolo to declare it separately

### Requirement: Run integration setup

The system SHALL provide `piccolo-gws setup` as a non-interactive diagnostic that reports setup state from inside a Piccolo setup container, and SHALL NOT write PicoClaw runtime configuration itself.

#### Scenario: Setup uses config directory
- **WHEN** setup is run without an explicit config directory
- **THEN** Google Workspace integration state is read from and written under `/root/.piccolo/integrations/gws`

#### Scenario: Setup patches PicoClaw config
- **WHEN** an action completes successfully and the MCP server entry is not yet registered
- **THEN** the action result carries a proposed configuration patch, and setup performs no patching itself
- **AND** Piccolo applies the proposal only after showing the diff and receiving confirmation

#### Scenario: Setup preserves user-owned config
- **WHEN** any setup or action path runs
- **THEN** `/root/.picoclaw/config.json` is not opened for writing, so unknown keys and unrelated user-owned sections are preserved by construction

#### Scenario: Setup backs up config
- **WHEN** setup or an action completes
- **THEN** no backup file is created by the integration, because only Piccolo writes `config.json` and Piccolo owns the backup convention

#### Scenario: Setup is a diagnostic, not a wizard
- **WHEN** `piccolo-gws setup` runs without a terminal
- **THEN** it reports state and the remaining required steps without rendering prompts or blocking on input

### Requirement: Provide stdio MCP server
The system SHALL provide `piccolo-gws mcp` as a stdio MCP server for PicoClaw.

#### Scenario: MCP server starts
- **WHEN** PicoClaw runs `piccolo-gws mcp --config-dir /root/.piccolo/integrations/gws --read-only`
- **THEN** the process speaks MCP over stdio and exposes Google Workspace tools

#### Scenario: Auth is missing
- **WHEN** the MCP server is invoked before Google auth is complete
- **THEN** Google Workspace tool calls return a clear login-required error referencing `/root/.piccolo/integrations/gws` without crashing the server

### Requirement: Expose read-only Gmail tools
The system SHALL expose curated read-only Gmail tools instead of arbitrary raw Google API calls.

#### Scenario: Search Gmail messages
- **WHEN** a Gmail search tool is called with a query and limit
- **THEN** the tool returns normalized message summaries matching the query

#### Scenario: Read Gmail message
- **WHEN** a Gmail read tool is called with a message identifier returned by search
- **THEN** the tool returns normalized message content and metadata

### Requirement: Expose read-only Calendar tools
The system SHALL expose curated read-only Calendar tools.

#### Scenario: List calendar events
- **WHEN** a calendar list tool is called with a time range
- **THEN** the tool returns normalized event summaries in that range

#### Scenario: Read calendar event
- **WHEN** a calendar read tool is called with an event identifier returned by list
- **THEN** the tool returns normalized event details

### Requirement: Expose read-only Drive tools
The system SHALL expose curated read-only Drive tools.

#### Scenario: Search Drive files
- **WHEN** a Drive search tool is called with a query and limit
- **THEN** the tool returns normalized file summaries matching the query

#### Scenario: Read Drive file
- **WHEN** a Drive read tool is called with a supported file identifier
- **THEN** the tool returns normalized file content or a clear unsupported-file response

### Requirement: Default to read-only safety
The system SHALL default to read-only Google Workspace behavior.

#### Scenario: Write operation requested
- **WHEN** a user or model attempts to perform a send, create, update, delete, or share operation through the initial integration
- **THEN** the integration refuses the operation because write tools are not available in the first version

### Requirement: Provide Google Workspace connectivity status
The system SHALL provide `piccolo-gws status --json` to test live read-only connectivity to Google Workspace services and emit Piccolo's generic integration status schema.

#### Scenario: Status schema is emitted
- **WHEN** `piccolo-gws status --json --config-dir /root/.piccolo/integrations/gws` is run
- **THEN** it emits JSON with schema `piccolo.integration.status.v1`, name `gws`, an overall status, a summary, and a list of checks

#### Scenario: Auth is valid
- **WHEN** Google auth is configured and `gws auth status` succeeds
- **THEN** the status output includes an `auth` check with status `ok`

#### Scenario: Auth is missing
- **WHEN** Google auth is missing, expired, or invalid
- **THEN** the status output includes an `auth` check with status `fail`
- **AND** Gmail, Calendar, and Drive checks are marked `skip`
- **AND** remediation tells the user to run `piccolo integration setup gws`

#### Scenario: Gmail connectivity succeeds
- **WHEN** auth is valid and a minimal read-only Gmail probe succeeds
- **THEN** the status output includes a `gmail` check with status `ok`

#### Scenario: Gmail connectivity fails
- **WHEN** auth is valid but the Gmail probe fails
- **THEN** the status output includes a `gmail` check with status `fail`
- **AND** the check includes an actionable message or remediation based on the backend error

#### Scenario: Calendar connectivity succeeds
- **WHEN** auth is valid and a minimal read-only Calendar probe succeeds
- **THEN** the status output includes a `calendar` check with status `ok`

#### Scenario: Calendar connectivity fails
- **WHEN** auth is valid but the Calendar probe fails
- **THEN** the status output includes a `calendar` check with status `fail`
- **AND** the check includes an actionable message or remediation based on the backend error

#### Scenario: Drive connectivity succeeds
- **WHEN** auth is valid and a minimal read-only Drive probe succeeds
- **THEN** the status output includes a `drive` check with status `ok`

#### Scenario: Drive connectivity fails
- **WHEN** auth is valid but the Drive probe fails
- **THEN** the status output includes a `drive` check with status `fail`
- **AND** the check includes an actionable message or remediation based on the backend error

#### Scenario: Backend command missing
- **WHEN** the packaged `gws` backend command is not available on PATH
- **THEN** the status output includes a backend check with status `fail`
- **AND** remediation explains that the Piccolo runtime image should be rebuilt or upgraded

#### Scenario: PicoClaw config path provided
- **WHEN** `piccolo-gws status --json` is run with `--picoclaw-config /root/.picoclaw/config.json`
- **THEN** the status output includes a `picoclaw-config` check verifying that `tools.mcp.servers.google_workspace` is registered with the expected command and config directory

#### Scenario: Overall status is derived from checks
- **WHEN** any required connectivity check has status `fail`
- **THEN** the overall status is `fail`
- **WHEN** no required checks fail and at least one check is `warn` or `unknown`
- **THEN** the overall status is `warn`
- **WHEN** all required checks are `ok` or non-failing `skip`
- **THEN** the overall status is `ok`

### Requirement: Gate login on existing credentials

The system SHALL NOT run `gws auth login` when no OAuth client credentials are configured, and SHALL explain the missing prerequisite instead.

#### Scenario: Login chosen without credentials
- **WHEN** the user selects "Login / authorize Google account" while neither `client_secret.json` nor a client id/secret environment pair exists
- **THEN** the system states that login requires OAuth credentials first and routes the user to import credentials or view manual instructions rather than running the login command

#### Scenario: Status reports token without credentials
- **WHEN** an authentication token state exists but OAuth client credentials are missing
- **THEN** setup reports that state as not usable for login/refresh and directs the user to import credentials

### Requirement: Avoid Google Cloud SDK as a runtime dependency

The system SHALL NOT require the Google Cloud SDK (`gcloud`) for any setup, status, or runtime path, and SHALL NOT include it in the flake package or release bundles.

#### Scenario: Setup runs without gcloud present
- **WHEN** setup runs in a runtime image where `gcloud` is not installed
- **THEN** every offered action still works and no command requiring `gcloud` is attempted

#### Scenario: Release bundle excludes Google Cloud SDK
- **WHEN** a release bundle is packaged
- **THEN** the bundle contains `piccolo-gws` and `gws` but no `gcloud` binary or Google Cloud SDK closure

### Requirement: Publish Piccolo integration release manifest
The project SHALL publish a machine-readable Piccolo integration release manifest with each GitHub Release intended for Piccolo binary installation.

#### Scenario: Release manifest exists
- **WHEN** a `piccolo-gws` GitHub Release is published
- **THEN** it contains a `piccolo-integration-release.json` asset
- **AND** the manifest schema is `piccolo.integration.release.v1`
- **AND** the manifest identifies package `github:kimus/piccolo-gws`

#### Scenario: Manifest lists platform artifacts
- **WHEN** the release manifest is read
- **THEN** it lists Linux amd64 and Linux arm64 artifacts
- **AND** each artifact entry includes platform, asset name, and SHA-256 digest

### Requirement: Publish Linux integration bundles
The project SHALL publish Linux integration bundle artifacts that contain every runtime command needed by the Piccolo integration descriptor.

#### Scenario: Linux amd64 bundle
- **WHEN** a release is published
- **THEN** it includes a Linux amd64 archive for Piccolo runtime images
- **AND** the archive contains executable `bin/piccolo-gws`
- **AND** the archive contains executable `bin/gws`

#### Scenario: Linux arm64 bundle
- **WHEN** a release is published
- **THEN** it includes a Linux arm64 archive for Piccolo runtime images
- **AND** the archive contains executable `bin/piccolo-gws`
- **AND** the archive contains executable `bin/gws`

#### Scenario: Bundle excludes Google Cloud SDK
- **WHEN** a release bundle is created
- **THEN** it does not include Google Cloud SDK or `gcloud`

### Requirement: Validate release bundles in CI
The release workflow SHALL validate each published bundle before uploading it.

#### Scenario: Descriptor smoke test
- **WHEN** a release bundle is assembled
- **THEN** CI runs `bin/piccolo-gws describe --json`
- **AND** validates that the command exits successfully with descriptor schema `piccolo.integration.v1`

#### Scenario: Status smoke test
- **WHEN** a release bundle is assembled
- **THEN** CI runs `bin/piccolo-gws status --config-dir <temp-dir> --json`
- **AND** verifies that the command emits `piccolo.integration.status.v1` JSON even when auth is not configured

### Requirement: Use repository-owned bare version tags
The project SHALL use bare version tags for its own releases.

#### Scenario: Publish version 3.1.0
- **WHEN** tag `3.1.0` is pushed
- **THEN** the release workflow publishes a GitHub Release for `3.1.0`
- **AND** generated `piccolo-gws` artifact metadata records version `3.1.0`
- **AND** no `v` prefix is added by the workflow

### Requirement: Preserve source-build development path
The project SHALL keep its Nix flake/source-build path available for development while release bundles serve the product install path.

#### Scenario: Nix build still works
- **WHEN** a developer runs `nix build .#`
- **THEN** the package builds `piccolo-gws` successfully
- **AND** `piccolo-gws describe --json` remains available from the built result

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

