## ADDED Requirements

### Requirement: Provide self-describing integration binary
The system SHALL provide a `piccolo-gws` executable with a `describe --json` command that returns a supported Piccolo integration descriptor.

#### Scenario: Descriptor command succeeds
- **WHEN** `piccolo-gws describe --json` is run
- **THEN** it prints JSON with schema `piccolo.integration.v1`, integration name `gws`, display name `Google Workspace`, and setup command metadata

#### Scenario: Descriptor includes setup command
- **WHEN** Piccolo reads the descriptor
- **THEN** the descriptor contains the command Piccolo must run for integration setup inside a setup container

### Requirement: Provide installable Nix flake package
The system SHALL be installable as a Nix flake package suitable for installation into a Piccolo gateway image.

#### Scenario: Package is installed
- **WHEN** the package reference `github:kimus/piccolo-gws` is installed during a Docker build
- **THEN** the `piccolo-gws` command is available on PATH in the resulting image

#### Scenario: Runtime dependencies are included
- **WHEN** `piccolo-gws` requires a backend such as `gws`
- **THEN** the package or wrapper provides the runtime dependency without requiring Piccolo to declare it separately

### Requirement: Run integration setup
The system SHALL provide `piccolo-gws setup` to configure Google Workspace integration state and PicoClaw runtime configuration from inside a Piccolo setup container.

#### Scenario: Setup uses config directory
- **WHEN** setup is run with `--config-dir /root/.picoclaw/integrations/google`
- **THEN** Google Workspace integration state is read from and written under that directory

#### Scenario: Setup patches PicoClaw config
- **WHEN** setup is confirmed by the user
- **THEN** setup merge-patches `/root/.picoclaw/config.json` to enable the Google Workspace MCP server

#### Scenario: Setup preserves user-owned config
- **WHEN** setup patches `config.json`
- **THEN** unknown keys and unrelated user-owned sections are preserved

#### Scenario: Setup backs up config
- **WHEN** setup writes `config.json`
- **THEN** it creates a backup before replacing the file

### Requirement: Provide stdio MCP server
The system SHALL provide `piccolo-gws mcp` as a stdio MCP server for PicoClaw.

#### Scenario: MCP server starts
- **WHEN** PicoClaw runs `piccolo-gws mcp --config-dir /root/.picoclaw/integrations/google --read-only`
- **THEN** the process speaks MCP over stdio and exposes Google Workspace tools

#### Scenario: Auth is missing
- **WHEN** the MCP server is invoked before Google auth is complete
- **THEN** Google Workspace tool calls return a clear login-required error without crashing the server

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
