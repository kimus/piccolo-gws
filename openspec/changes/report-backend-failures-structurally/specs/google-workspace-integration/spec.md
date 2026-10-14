## ADDED Requirements

### Requirement: Report backend failures from the structured response

The system SHALL derive a reported failure from the provider's structured error fields — reason, message, status code and any supplied remediation link — and SHALL fall back to free text only when no structured response exists.

#### Scenario: A disabled API

- **WHEN** the backend fails with an `accessNotConfigured` response while stderr carries an unrelated informational banner
- **THEN** the reported message is the provider's explanation, not the banner
- **AND** the supplied enable link is carried as remediation

#### Scenario: Envelope surrounded by noise

- **WHEN** stdout mixes banner lines with a JSON error object
- **THEN** the object is still located and parsed

### Requirement: Login prompts come from authentication signals only

The system SHALL NOT decide that reauthentication is the remedy by searching human-readable text for authentication-related words.

#### Scenario: Authorization wording in an unrelated error

- **WHEN** a failure message contains the word "authorization" but the response is not an authentication challenge
- **THEN** login is not reported as required

#### Scenario: Genuine unauthenticated response

- **WHEN** the provider answers 401 or an unauthenticated reason
- **THEN** login is reported as required

### Requirement: Connectivity diagnosis names what failed

The system SHALL report which services were unreachable and how many were reachable, rather than a single generic failure sentence.

#### Scenario: One service disabled among healthy ones

- **WHEN** Gmail and Calendar respond but Drive is not enabled for the project
- **THEN** the summary says one of three is unreachable and names Drive
- **AND** the remediation is the enable link rather than advice to widen scopes

### Requirement: Every action exit ends the event stream

The system SHALL emit a terminal event on every failure path, including an error that escapes the action handler, so a host can distinguish a report from a crash.

#### Scenario: Unexpected internal error

- **WHEN** an action fails in a way no handler described
- **THEN** an `error` event with code `internal` is emitted before exiting
- **AND** the exit is not silent

## MODIFIED Requirements

### Requirement: Invoke actions by id with structured streaming

The system SHALL run one declared action per invocation, reading answers from stdin and writing NDJSON events to stdout, and SHALL end every invocation with a terminal event whether the action succeeded, reported a failure, or died unexpectedly.

#### Scenario: Success ends with a result

- **WHEN** an action completes
- **THEN** a `result` event carries the status and any proposed configuration patch

#### Scenario: Failure ends with an error event

- **WHEN** an action cannot complete
- **THEN** an `error` event carries a machine code, a human message, and remediation or a link when one is known

#### Scenario: Crash is distinguishable from a report

- **WHEN** the process dies without emitting a terminal event
- **THEN** the host can tell that outcome apart from a reported failure by the absence of the event
