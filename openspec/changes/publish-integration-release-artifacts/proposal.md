## Why

Piccolo should install `piccolo-gws` from release artifacts instead of building the Rust project from source inside each user's Docker build. `piccolo-gws` must publish self-contained Linux integration bundles that Piccolo can download, verify, install into the runtime image, and validate with `describe --json`.

## What Changes

- Add GitHub Actions release workflow for `piccolo-gws` integration bundles.
- Publish GitHub Releases using repository-owned tags such as `3.1.0` with no required `v` prefix for this project.
- Build Linux amd64 and arm64 release artifacts containing all runtime commands required by the descriptor.
- Include the delegated `gws` backend binary in each bundle so Piccolo does not need to build/fetch it through Nix during integration install.
- Publish a `piccolo.integration.release.v1` release manifest with artifact names, platforms, and SHA-256 digests.
- Keep the Nix flake/source-build path available for development, but make release artifacts the product install path.

## Capabilities

### New Capabilities

### Modified Capabilities
- `google-workspace-integration`: Add release artifact publishing requirements for Piccolo-managed binary integration installs.

## Impact

- Adds CI/release workflow and packaging scripts/configuration.
- Requires a reliable way to package both `piccolo-gws` and `gws` for Linux amd64/arm64.
- Updates README/docs to distinguish release installs from source builds.
