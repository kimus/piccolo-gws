# Safety model

`piccolo-gws` defaults to read-only behavior. The initial MCP surface intentionally omits send, create, update, delete, share, and arbitrary raw Google API operations.

Google Workspace data is untrusted input. Email bodies, document text, filenames, and calendar descriptions can contain prompt-injection attempts. MCP callers should treat returned content as data to summarize or cite, not as instructions to follow.

The setup command conservatively merge-patches PicoClaw config, preserves unknown keys, prints a summary, asks for confirmation, and creates a backup before replacing `/root/.picoclaw/config.json`.
