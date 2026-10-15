## 1. Overlap the read with the wait

- [x] 1.1 `run_gws` takes both output streams before waiting and drains each on a worker thread
- [x] 1.2 The 20 s timeout still bounds the child; drained bytes are reassembled into the `Output` the failure parser consumes
- [x] 1.3 `PIPE_DRAIN_GRACE` (5 s) turns a stream held open by a grandchild into a named error instead of a new hang
- [x] 1.4 `stdin(null)` on the tool spawn site and the `auth status` probe

## 2. Pin the shape with a test that fails the old way

- [x] 2.1 `mcp_tool_call_survives_a_response_larger_than_the_pipe_buffer`: a 200 KiB single-line JSON response through the real `tools/call` path
- [x] 2.2 Verified against the pre-fix code: **fails in 20.26 s** with `"ok": false`; **passes in 0.43 s** after
- [x] 2.3 Asserts the payload arrives intact (`snippet` > 64 KiB), not just that no error was returned
- [x] 2.4 Existing suite green: 45 unit + 17 integration tests, `cargo fmt --check` clean

## 3. Release and proof

- [x] 3.1 0.3.6 released for both platforms (published 2026-10-15T17:42Z, Release run 38779357020 success)
- [x] 3.2 Adopted into the workspace (`piccolo integration upgrade`) and promoted through the executability gate — image `cd4c6f2a1baa` -> `8c559b1b9716`, recorded as `previous_image`, `piccolo doctor` shows `[OK] running container image: piccolo/picoclaw-runtime:8c559b1b9716` and `[OK] mcp: 1 server(s) reachable by coordinator "main" (google_workspace)`
- [x] 3.3 The three messages that reproduced the failure, read through `piccolo-gws mcp` over the real `tools/call` path inside the running gateway, authenticated with the workspace's own credentials:

  | message id | result | wall time | response bytes |
  |---|---|---|---|
  | `1a14036c264927c2` | ok, no timeout | 651 ms | 97,373 |
  | `1a1405b80d603306` | ok, no timeout | 627 ms | 55,570 |
  | `1a14061499418bbe` | ok, no timeout | 564 ms | 75,060 |

  Measured at the MCP layer rather than through `piccolo chat`, so the agent's own retries are not in these numbers — what is measured is the exact call that used to cost 20,403 ms.
- [x] 3.4 The previously-timed-out large messages (74 KiB and 96 KiB, read straight from the API) both return whole: no response above the 64 KiB pipe buffer fails, and each one crosses the pipe complete rather than truncated
