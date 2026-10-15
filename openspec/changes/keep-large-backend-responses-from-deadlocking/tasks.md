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

- [ ] 3.1 0.3.6 released for both platforms
- [ ] 3.2 Adopted into the workspace (`piccolo integration upgrade`) and promoted through the executability gate
- [ ] 3.3 Live `piccolo chat -m "read my latest emails"` against the inbox that reproduced it: no `backend_timeout`, and the turn finishes in seconds rather than minutes
- [ ] 3.4 Confirm the previously-timed-out large messages (74 KiB, 96 KiB) now return — they are the exact inputs that failed
