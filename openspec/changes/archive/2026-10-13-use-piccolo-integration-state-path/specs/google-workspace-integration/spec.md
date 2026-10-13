## MODIFIED Requirements

### Requirement: Provide self-describing integration binary
The system SHALL provide a `piccolo-gws` executable with a `describe --json` command that returns a supported Piccolo integration descriptor using Piccolo-owned integration state paths.

#### Scenario: Descriptor command succeeds
- **WHEN** `piccolo-gws describe --json` is run
- **THEN** it prints JSON with schema `piccolo.integration.v1`, integration name `gws`, display name `Google Workspace`, and setup command metadata

#### Scenario: Descriptor uses Piccolo integration state path
- **WHEN** Piccolo reads the descriptor
- **THEN** the descriptor setup and MCP command metadata use `/root/.piccolo/integrations/gws` as the default `--config-dir`

### Requirement: Run integration setup
The system SHALL provide `piccolo-gws setup` to configure Google Workspace integration state and PicoClaw runtime configuration from inside a Piccolo setup container.

#### Scenario: Setup uses config directory
- **WHEN** setup is run without an explicit config directory
- **THEN** Google Workspace integration state is read from and written under `/root/.piccolo/integrations/gws`

#### Scenario: Setup patches PicoClaw config
- **WHEN** setup is confirmed by the user
- **THEN** setup merge-patches `/root/.picoclaw/config.json` to enable the Google Workspace MCP server using `piccolo-gws mcp --config-dir /root/.piccolo/integrations/gws --read-only`

#### Scenario: Setup preserves user-owned config
- **WHEN** setup patches `config.json`
- **THEN** unknown keys and unrelated user-owned sections are preserved

#### Scenario: Setup backs up config
- **WHEN** setup writes `config.json`
- **THEN** it creates a backup before replacing the file

### Requirement: Provide stdio MCP server
The system SHALL provide `piccolo-gws mcp` as a stdio MCP server for PicoClaw.

#### Scenario: MCP server starts
- **WHEN** PicoClaw runs `piccolo-gws mcp --config-dir /root/.piccolo/integrations/gws --read-only`
- **THEN** the process speaks MCP over stdio and exposes Google Workspace tools

#### Scenario: Auth is missing
- **WHEN** the MCP server is invoked before Google auth is complete
- **THEN** Google Workspace tool calls return a clear login-required error referencing `/root/.piccolo/integrations/gws`
