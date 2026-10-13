## 1. Release Bundle Format

- [x] 1.1 Define release bundle layout with `bin/piccolo-gws`, `bin/gws`, and optional diagnostic metadata
- [x] 1.2 Define `piccolo.integration.release.v1` manifest generation with package, version, artifacts, platforms, and SHA-256 digests
- [x] 1.3 Choose archive format and asset naming for Linux amd64 and Linux arm64 bundles
- [x] 1.4 Ensure bundle metadata uses bare project versions such as `3.1.0`

## 2. Build and Package Runtime Commands

- [x] 2.1 Build `piccolo-gws` release binaries for Linux amd64 and Linux arm64
- [x] 2.2 Package the delegated `gws` backend binary for Linux amd64 and Linux arm64
- [ ] 2.3 Include Google Cloud SDK (`gcloud`) in bundles so `gws auth setup` can create credentials in-runtime
- [x] 2.4 Make the packaging workflow fail if any descriptor-declared runtime command is missing

## 3. GitHub Actions Release Workflow

- [x] 3.1 Add release workflow triggered by bare version tags
- [x] 3.2 Assemble platform-specific archives
- [x] 3.3 Generate SHA-256 digests and `piccolo-integration-release.json`
- [x] 3.4 Smoke-test `bin/piccolo-gws describe --json` from each bundle before upload
- [x] 3.5 Smoke-test `bin/piccolo-gws status --config-dir <temp-dir> --json` from each bundle before upload
- [x] 3.6 Upload archives and manifest to the GitHub Release

## 4. Documentation and Validation

- [x] 4.1 Update README to document release bundles as the product install path for Piccolo
- [ ] 4.2 Document that release bundles ship `gcloud`, so `gws auth setup` and `gws auth login` both run in the Piccolo runtime
- [x] 4.3 Keep Nix flake build/test documentation for source-build development
- [x] 4.4 Run `nix develop -c cargo fmt --check`, `nix develop -c cargo test`, `nix build .#`, and `openspec validate publish-integration-release-artifacts --strict`
