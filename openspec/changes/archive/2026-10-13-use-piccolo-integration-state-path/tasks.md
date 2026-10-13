## 1. Path constants

- [x] 1.1 Define a single default config/state path constant: `/root/.piccolo/integrations/gws`.
- [x] 1.2 Use the constant for CLI defaults, descriptor metadata, setup, MCP, backend messages, docs, and tests.

## 2. Descriptor and config patch

- [x] 2.1 Update `piccolo-gws describe --json` setup command metadata to use `/root/.piccolo/integrations/gws`.
- [x] 2.2 Update `piccolo-gws describe --json` MCP command metadata to use `/root/.piccolo/integrations/gws`.
- [x] 2.3 Update setup's PicoClaw `config.json` patch to register the MCP command with the new path.

## 3. Runtime behavior

- [x] 3.1 Update setup/auth output and environment handling to use the new default state path.
- [x] 3.2 Update backend login-required errors to reference the new path.
- [x] 3.3 Preserve explicit `--config-dir` override behavior.

## 4. Documentation and tests

- [x] 4.1 Update README and docs to replace `/root/.picoclaw/integrations/google` with `/root/.piccolo/integrations/gws`.
- [x] 4.2 Update tests asserting descriptor paths, setup config patches, and error messages.

## 5. Verification

- [x] 5.1 Run the available Rust test suite.
- [x] 5.2 Validate `use-piccolo-integration-state-path` OpenSpec artifacts.
