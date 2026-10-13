## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Present a guided interactive setup menu

The system SHALL present an interactive guided setup menu using a terminal prompt library when `piccolo-gws setup` runs attached to a TTY and `--yes` is not given. The menu SHALL offer a recommended flow that completes Google Workspace access and SHALL offer advanced single-purpose actions.

#### Scenario: Menu lists guided and advanced actions
- **WHEN** interactive setup starts
- **THEN** the menu offers at least: "Set up Google Workspace access", "Create credentials", "Import credentials", "Login / authorize Google account", "Manual instructions", and "Skip"

#### Scenario: Auth state is reported before acting
- **WHEN** interactive setup starts
- **THEN** the system reports whether OAuth client credentials are present and whether a Google account login/token exists, using neutral language that does not call a token-only state "configured" while credentials are missing

### Requirement: Complete the guided access flow in prerequisite order

The system SHALL implement "Set up Google Workspace access" as an ordered flow that ensures OAuth client credentials exist before attempting Google account authorization, then proceeds to the PicoClaw configuration step.

#### Scenario: Credentials missing, gcloud available
- **WHEN** the guided flow runs and no OAuth client credentials are configured and `gcloud` is available
- **THEN** the system offers to create credentials by running `gws auth setup` and allows the user to select or create a Google Cloud project through it

#### Scenario: Credentials missing, gcloud unavailable
- **WHEN** the guided flow runs and no OAuth client credentials are configured and `gcloud` is not available
- **THEN** the system explains that credential creation requires Google Cloud SDK and offers to import pasted credentials or show manual instructions instead, without running a command that is guaranteed to fail

#### Scenario: Credentials already present
- **WHEN** the guided flow runs and OAuth client credentials are already configured
- **THEN** the system skips credential creation and proceeds to Google account authorization

#### Scenario: Login succeeds after credentials exist
- **WHEN** OAuth client credentials exist and the user authorizes the Google account
- **THEN** the system runs `gws auth login` with inherited stdio so the browser OAuth URL and instructions reach the user, then re-checks and reports authentication status

#### Scenario: Guided flow finishes with configuration
- **WHEN** the guided flow reaches credentials satisfied and authorization satisfied or deliberately skipped
- **THEN** the system continues to the existing PicoClaw config preview and patch step

### Requirement: Create credentials through Google Cloud SDK

The system SHALL make `gcloud` available in the packaged integration runtime and SHALL expose a "Create credentials" action that configures a Google Cloud project and OAuth client via `gws auth setup`.

#### Scenario: gcloud is present in the runtime image
- **WHEN** a Piccolo runtime image is built from a `piccolo-gws` release bundle
- **THEN** `gcloud` is available on PATH for setup-time credential creation

#### Scenario: Create credentials succeeds
- **WHEN** the user selects "Create credentials" and `gws auth setup` completes successfully
- **THEN** the system reports that OAuth client credentials are configured and offers Google account authorization

#### Scenario: Create credentials fails or is declined
- **WHEN** `gws auth setup` exits non-zero or the user declines project configuration
- **THEN** the system reports the failure without hiding `gws`/`gcloud` output and offers to import credentials or show manual instructions

### Requirement: Import OAuth client credentials by pasting

The system SHALL provide an "Import credentials" action that accepts pasted `client_secret.json` content, validates that it is a plausible OAuth client secret, and stores it in the integration auth state directory.

#### Scenario: Valid pasted secret is stored
- **WHEN** the user pastes JSON that contains an OAuth client id and client secret
- **THEN** the system writes it to `<state-dir>/auth/client_secret.json` and reports that credentials are configured without echoing the secret back

#### Scenario: Invalid pasted content is rejected
- **WHEN** the pasted content is not valid JSON or lacks client id/secret fields
- **THEN** the system rejects it, explains what a `client_secret.json` should contain, and does not write the file

#### Scenario: Import replaces existing credentials only after confirmation
- **WHEN** credentials already exist and the user chooses to import new ones
- **THEN** the system asks for confirmation before overwriting the existing client secret file

### Requirement: Gate login on existing credentials

The system SHALL NOT run `gws auth login` when no OAuth client credentials are configured, and SHALL explain the missing prerequisite instead.

#### Scenario: Login chosen without credentials
- **WHEN** the user selects "Login / authorize Google account" while neither `client_secret.json` nor client id/secret environment configuration exists
- **THEN** the system states that login requires OAuth credentials first and routes the user to create or import credentials rather than running the login command

#### Scenario: Status reports token without credentials
- **WHEN** an authentication token state exists but OAuth client credentials are missing
- **THEN** setup reports that state as not usable for login/refresh and directs the user to create or import credentials

### Requirement: Provide manual instructions and skip

The system SHALL provide a "Manual instructions" action that explains how to create an OAuth client in Google Cloud Console and where credentials must be placed, and SHALL provide a "Skip" action that leaves authentication unconfigured without failing setup.

#### Scenario: Manual instructions name the exact destination
- **WHEN** the user selects "Manual instructions"
- **THEN** the system prints Google Cloud Console steps and the container-side and host-side paths for `client_secret.json`, plus the accepted environment variable alternative

#### Scenario: Skip leaves tools reporting login required
- **WHEN** the user selects "Skip"
- **THEN** setup continues without configuring authentication, does not return an error, and states that Google Workspace tools will report login-required until auth is completed

### Requirement: Keep non-interactive setup predictable

The system SHALL keep `--yes` non-interactive: it MUST NOT render prompt UI, MUST NOT require credential-creating or login steps, and MUST still apply the PicoClaw configuration patch.

#### Scenario: Yes mode with auth already configured
- **WHEN** `piccolo-gws setup --yes` runs with OAuth credentials and a valid Google account token
- **THEN** the system skips all auth prompts and applies the PicoClaw config patch after showing the summary

#### Scenario: Yes mode without auth configured
- **WHEN** `piccolo-gws setup --yes` runs without Google authentication configured
- **THEN** the system applies the PicoClaw config patch, reports that authentication is still required, and prints how to complete it interactively
