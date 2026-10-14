# gws-backend-invocation Specification

## Purpose
TBD - created by archiving change fix-gws-backend-grammar. Update Purpose after archive.
## Requirements
### Requirement: Invoke the delegated backend with the verified gws grammar

The system SHALL invoke the delegated `gws` backend using the grammar `gws <service> <resource> [sub-resource] <method> --params <JSON>` as verified against the pinned `gws` version, and SHALL NOT construct commands from unverified subcommand names.

#### Scenario: Gmail search maps to the real resource method
- **WHEN** the `gmail_search` tool is called with a query
- **THEN** the system runs `gws gmail users messages list` with a `--params` JSON object containing `userId`, `q`, and `maxResults`

#### Scenario: Gmail read maps to the real resource method
- **WHEN** the `gmail_read` tool is called with a message id
- **THEN** the system runs `gws gmail users messages get` with `--params` containing `userId` and `id`

#### Scenario: Calendar list maps to the real resource method
- **WHEN** the `calendar_list` tool is called with a time range
- **THEN** the system runs `gws calendar events list` with `--params` containing `calendarId`, `timeMin`, and `timeMax`

#### Scenario: Calendar read uses the calendar parameter name
- **WHEN** the `calendar_read` tool is called with an event id
- **THEN** the system runs `gws calendar events get` with `--params` containing `calendarId` and `eventId`

#### Scenario: Drive search and read map to the real resource methods
- **WHEN** `drive_search` or `drive_read` is called
- **THEN** the system runs `gws drive files list` with `q`/`pageSize` or `gws drive files get` with `fileId` respectively

### Requirement: Supply required API parameters with documented defaults

The system SHALL supply backend-required parameters that the tool surface does not expose, using defaults consistent with a single-user read-only assistant, and SHALL allow overrides where the tool already accepts them.

#### Scenario: Gmail defaults to the authenticated user
- **WHEN** a Gmail tool call omits a user
- **THEN** the system uses `userId` `me`

#### Scenario: Calendar defaults to the primary calendar
- **WHEN** a Calendar tool call omits a calendar
- **THEN** the system uses `calendarId` `primary`

### Requirement: Validate backend grammar during release packaging

The release packaging step SHALL verify that every mapped backend method exists in the packaged `gws` version before publishing a bundle.

#### Scenario: Method missing from the pinned backend
- **WHEN** a mapped method is not resolvable via `gws schema`
- **THEN** packaging fails and names the method that no longer exists

#### Scenario: Mapped methods resolve
- **WHEN** all mapped methods resolve and a dry-run call validates
- **THEN** packaging continues to produce the release bundle

### Requirement: Derive authentication state from the backend payload

The system SHALL determine Google authentication state from the JSON reported by `gws auth status` and SHALL NOT treat its process exit code as proof of authentication, because the backend exits 0 when no credentials exist.

#### Scenario: Backend exits zero without credentials
- **WHEN** `gws auth status` exits 0 and reports `auth_method` `none`
- **THEN** the system reports Google auth as not configured
- **AND** service checks are skipped rather than reported as successful

#### Scenario: Backend reports usable credentials
- **WHEN** `gws auth status` reports a non-`none` `auth_method`
- **THEN** the system treats Google Workspace as authenticated and runs live service checks

#### Scenario: Login-required guidance names the real state directory
- **WHEN** a tool or probe reports login-required
- **THEN** the remediation references the integration's actual state directory rather than a hard-coded default path

### Requirement: Status probes reflect real service reachability

The system's connectivity status checks SHALL use the same corrected backend invocations as the tools, so a check failure indicates an authentication, scope, or API problem rather than a command-line parse error.

#### Scenario: Authenticated probe reaches the API
- **WHEN** Google auth is valid and the Gmail API is reachable
- **THEN** the Gmail status check reports success using a real list call

#### Scenario: Unauthenticated probe reports login required
- **WHEN** Google auth is not configured
- **THEN** service checks are skipped with a login-required explanation rather than reporting a grammar error

