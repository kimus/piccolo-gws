## Context

The initial `piccolo-gws` implementation used `/root/.picoclaw/integrations/google` as its default state path. Piccolo's structured workspace design now separates ownership more clearly: `/root/.picoclaw` is PicoClaw's home, while `/root/.piccolo` is Piccolo's state root. Integration state is also keyed by integration descriptor name, so Google Workspace's integration name `gws` should use `gws`, not `google`, in paths.

## Goals / Non-Goals

**Goals:**

- Use `/root/.piccolo/integrations/gws` as the default state/config directory in descriptor output, setup, MCP command examples, backend error messages, docs, and tests.
- Ensure `piccolo-gws setup` writes MCP config that calls `piccolo-gws mcp --config-dir /root/.piccolo/integrations/gws --read-only`.
- Keep command-line `--config-dir` override behavior unchanged for tests and advanced users.
- Align host state with Piccolo's mount model: `<workspace>/integrations/gws`.

**Non-Goals:**

- Implement Piccolo's workspace migration in this repo.
- Automatically migrate old `integrations/google` state.
- Rename the integration itself; descriptor name remains `gws`.

## Decisions

### Default path is a single constant

Introduce or centralize a default config dir constant, e.g. `/root/.piccolo/integrations/gws`, and use it in CLI defaults, descriptor metadata, config patching defaults, docs, and tests. This avoids drift between `describe`, setup, and MCP.

### Descriptor name controls directory name

The integration descriptor name remains `gws`, and the default state directory uses the same name. This matches Piccolo's rule that `<workspace>/integrations/<name>` maps to `/root/.piccolo/integrations/<name>`.

### No automatic state migration in piccolo-gws

Old workspaces are still experimental. Migration should be driven by Piccolo's workspace-layout migration, not by provider integrations. If old state exists under `/root/.picoclaw/integrations/google`, users can re-run setup or manually copy state if needed.

## Risks / Trade-offs

- **Existing local test state becomes orphaned** → Document the path change and tell users to rerun setup.
- **Piccolo and piccolo-gws path constants can drift** → Keep the path in `describe --json` and have Piccolo consume the descriptor rather than hardcoding Google-specific paths.
- **gws auth state may need to be recreated** → Setup remains interactive and can run `gws auth setup` or `gws auth login` in the new auth directory.
