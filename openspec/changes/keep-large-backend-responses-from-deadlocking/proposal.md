## Why

A real inbox call reproduced this in the running gateway:

```
Tool execution failed error="MCP tool returned error: {"error":{"code":"backend_timeout",...}}" duration=20403 tool=mcp_google_workspace_gmail_read
```

The same `gws` invocation, run by hand in that container with stdout going to a file, returned in **274 ms**. The 20 s figure was not Google's latency.

`run_gws` piped the child's stdout and stderr, then waited for the child to exit **before reading either stream**. A Gmail message read is one JSON document as big as the message — 74 KiB and 96 KiB for two real messages in that inbox, both past the ~64 KiB OS pipe buffer. So the child filled the pipe and blocked in `write`, the parent sat in `wait` waiting for a child blocked on the parent, and the timeout fired. Every message over the buffer size failed after 20 s while small ones succeeded, which is why the agent still answered: it got the short mails and timed out on the long ones.

The reported error was also wrong in kind. `backend_timeout` points the user at their network; the defect was in how this process read its own child.

## What Changes

- Both output streams are drained on worker threads **while** the wait runs, so the child always has somewhere to write and the timeout measures the work, not the pipe.
- The timeout still bounds the call, and the drained bytes are reassembled into the same `Output` the failure parser already expects — no change to error classification.
- A drained stream that never closes after the child is gone becomes its own error instead of hanging: a 5 s grace, then `backend stdout never closed after the process exited`. Killing the direct child does not close a write end held by a grandchild.
- The backend's stdin is now `/dev/null` at both spawn sites. This process is an MCP server whose stdin carries JSON-RPC framing; an inherited stdin lets a child consume bytes addressed to the server.

## Capabilities

### New Capabilities

### Modified Capabilities

- `gws-backend-invocation`: a large backend response SHALL be delivered whole, and SHALL NOT be reported as a timeout

## Impact

- `src/backend.rs` — `run_gws` read/write overlap, `drain_pipe`/`collect_drained`, `stdin(null)` on both spawn sites
- Tests: `mcp_tool_call_survives_a_response_larger_than_the_pipe_buffer` (200 KiB response through the real MCP `tools/call` path)
- Release: 0.3.6. This fix cannot reach a workspace any other way — the binary is unpacked into the runtime image at build time, and Piccolo resolves integrations from GitHub releases only (source-built integrations are retired).
