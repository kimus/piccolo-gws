# Backend decision

Initial backend: shell out to a `gws` command-line backend from `piccolo-gws`.

Rationale:

- Keeps the Piccolo/MCP contract stable while Google auth details evolve.
- Avoids exposing raw `gws` commands to models; only curated read-only MCP tools call the backend.
- Allows future replacement with direct Rust Google Workspace APIs without changing `piccolo-gws describe`, setup, or MCP tool schemas.

Backend calls are wrapped by `src/backend.rs`, which normalizes JSON/text output and maps auth failures into a stable `login_required` error shape.
