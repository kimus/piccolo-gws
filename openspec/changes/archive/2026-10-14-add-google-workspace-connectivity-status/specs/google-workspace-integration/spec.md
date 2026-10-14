## MODIFIED Requirements

### Requirement: Provide self-describing integration binary
The system SHALL provide a `piccolo-gws` executable with a `describe --json` command that returns a supported Piccolo integration descriptor using Piccolo-owned integration state paths, including setup, MCP, and status command metadata.

#### Scenario: Descriptor command succeeds
- **WHEN** `piccolo-gws describe --json` is run
- **THEN** it prints JSON with schema `piccolo.integration.v1`, integration name `gws`, display name `Google Workspace`, setup command metadata, MCP command metadata, and status command metadata

#### Scenario: Descriptor uses Piccolo integration state path
- **WHEN** Piccolo reads the descriptor
- **THEN** the descriptor setup, MCP, and status command metadata use `/root/.piccolo/integrations/gws` as the default `--config-dir`

## ADDED Requirements

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
