## ADDED Requirements

### Requirement: Publish Piccolo integration release manifest
The project SHALL publish a machine-readable Piccolo integration release manifest with each GitHub Release intended for Piccolo binary installation.

#### Scenario: Release manifest exists
- **WHEN** a `piccolo-gws` GitHub Release is published
- **THEN** it contains a `piccolo-integration-release.json` asset
- **AND** the manifest schema is `piccolo.integration.release.v1`
- **AND** the manifest identifies package `github:kimus/piccolo-gws`

#### Scenario: Manifest lists platform artifacts
- **WHEN** the release manifest is read
- **THEN** it lists Linux amd64 and Linux arm64 artifacts
- **AND** each artifact entry includes platform, asset name, and SHA-256 digest

### Requirement: Publish Linux integration bundles
The project SHALL publish Linux integration bundle artifacts that contain every runtime command needed by the Piccolo integration descriptor.

#### Scenario: Linux amd64 bundle
- **WHEN** a release is published
- **THEN** it includes a Linux amd64 archive for Piccolo runtime images
- **AND** the archive contains executable `bin/piccolo-gws`
- **AND** the archive contains executable `bin/gws`

#### Scenario: Linux arm64 bundle
- **WHEN** a release is published
- **THEN** it includes a Linux arm64 archive for Piccolo runtime images
- **AND** the archive contains executable `bin/piccolo-gws`
- **AND** the archive contains executable `bin/gws`

#### Scenario: Bundle includes Google Cloud SDK
- **WHEN** a release bundle is created
- **THEN** it includes `gcloud` so `gws auth setup` can create credentials inside the Piccolo runtime
- **AND** the Google Cloud SDK ships as part of the bundle runtime closure rather than being downloaded per workspace

### Requirement: Validate release bundles in CI
The release workflow SHALL validate each published bundle before uploading it.

#### Scenario: Descriptor smoke test
- **WHEN** a release bundle is assembled
- **THEN** CI runs `bin/piccolo-gws describe --json`
- **AND** validates that the command exits successfully with descriptor schema `piccolo.integration.v1`

#### Scenario: Status smoke test
- **WHEN** a release bundle is assembled
- **THEN** CI runs `bin/piccolo-gws status --config-dir <temp-dir> --json`
- **AND** verifies that the command emits `piccolo.integration.status.v1` JSON even when auth is not configured

### Requirement: Use repository-owned bare version tags
The project SHALL use bare version tags for its own releases.

#### Scenario: Publish version 3.1.0
- **WHEN** tag `3.1.0` is pushed
- **THEN** the release workflow publishes a GitHub Release for `3.1.0`
- **AND** generated `piccolo-gws` artifact metadata records version `3.1.0`
- **AND** no `v` prefix is added by the workflow

### Requirement: Preserve source-build development path
The project SHALL keep its Nix flake/source-build path available for development while release bundles serve the product install path.

#### Scenario: Nix build still works
- **WHEN** a developer runs `nix build .#`
- **THEN** the package builds `piccolo-gws` successfully
- **AND** `piccolo-gws describe --json` remains available from the built result
