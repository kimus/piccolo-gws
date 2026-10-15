## ADDED Requirements

### Requirement: Deliver backend responses larger than the OS pipe buffer

The system SHALL read a backend child's stdout and stderr concurrently with waiting for it to exit, so that a response larger than the operating system's pipe buffer is delivered whole and is never reported as a timeout.

#### Scenario: A full Gmail message is read

- **WHEN** `gmail_read` returns a JSON document larger than the pipe buffer
- **THEN** the response reaches the caller intact
- **AND** no timeout error is produced

#### Scenario: A fast call stays fast

- **WHEN** the backend exits successfully within its timeout
- **THEN** the call returns with the child's exit status and both output streams

#### Scenario: Output stops arriving after the child is gone

- **WHEN** the backend exits but a stream is kept open by another process
- **THEN** the call ends with an error naming which stream never closed
- **AND** it does not hang indefinitely

### Requirement: Never expose the MCP server's stdin to the backend

The system SHALL NOT let a backend child inherit this process's stdin, because that stream carries JSON-RPC framing addressed to the MCP server.

#### Scenario: Backend is invoked from an MCP tool call

- **WHEN** a tool call spawns the backend
- **THEN** the child's stdin is at end-of-file from the start
- **AND** pending and future requests remain readable by the server
