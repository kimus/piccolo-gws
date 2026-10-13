## Context

`piccolo-gws` currently exposes the right runtime commands (`describe`, `setup`, `status`, `mcp`) but users install it through Piccolo's Devbox/Nix source-build path. That path can download Rust build dependencies and the `gws` runtime closure during Docker builds. The desired integration UX is release-based: Piccolo downloads a verified bundle and copies binaries into the runtime image.

The release bundle must include every command needed by the descriptor, especially `piccolo-gws` and the delegated `gws` backend. It now also includes Google Cloud SDK (`gcloud`): guided credential creation (`gws auth setup`) is a first-class setup path, so `gcloud` must exist inside the Piccolo runtime. This reverses the earlier size-driven decision; see `add-guided-google-workspace-setup`.

## Goals / Non-Goals

**Goals:**

- Publish Linux amd64 and arm64 release bundles for Piccolo.
- Include `piccolo-gws` and `gws` runtime binaries in each bundle.
- Publish a machine-readable `piccolo.integration.release.v1` manifest with digests.
- Use bare project tags such as `3.1.0` for this repository.
- Keep `piccolo-gws describe --json` as the runtime descriptor truth after install.

**Non-Goals:**

- No host Piccolo CLI release from this repository.
- Google Cloud SDK ships in integration bundles so setup can create OAuth credentials.
- No change to read-only MCP tool safety.
- No direct replacement of `gws` with native Google API implementation in this change.

## Decisions

### Bundle layout

Each archive contains:

```text
bin/
  piccolo-gws
  gws
share/piccolo/integration/
  bundle.json
```

`bundle.json` is local metadata for diagnostics. Piccolo core relies primarily on the GitHub release manifest and post-install descriptor validation.

### Release manifest

Each GitHub Release includes `piccolo-integration-release.json`:

```json
{
  "schema": "piccolo.integration.release.v1",
  "package": "github:kimus/piccolo-gws",
  "version": "3.1.0",
  "artifacts": [
    {
      "platform": "linux/amd64",
      "asset": "piccolo-gws_3.1.0_linux_amd64.tar.gz",
      "sha256": "..."
    },
    {
      "platform": "linux/arm64",
      "asset": "piccolo-gws_3.1.0_linux_arm64.tar.gz",
      "sha256": "..."
    }
  ]
}
```

### Building the delegated backend

The package needs a reproducible way to obtain Linux `gws` binaries for amd64 and arm64. Viable approaches:

1. Use Nix in CI to build/copy `gws` for target Linux platforms when supported.
2. Download upstream `gws` release artifacts if the upstream project publishes suitable static binaries.
3. Temporarily use Nix-built Linux artifacts and revisit native backend implementation later.

The workflow should fail if `gws` cannot be packaged for a target platform.

### Versioning

This repository uses bare tags such as `3.1.0`. Piccolo treats explicit package selectors as opaque, so other third-party integrations can use different tag schemes such as `v2.1.10` without affecting `piccolo-gws`.

### Runtime auth behavior

Release bundles include `gws` and `gcloud`. Bundling `gcloud` grows the artifact considerably, which is accepted in exchange for a working guided "Create credentials" flow. A setup-only runtime image is the follow-up mitigation if size becomes a problem.

## Risks / Trade-offs

- **Cross-platform backend packaging complexity** → Keep the workflow explicit and fail hard when `gws` is missing for a target platform.
- **Large bundles because Google Cloud SDK is included** → Record bundle sizes per release and revisit a setup-only image or on-demand provisioning layer.
- **Digest manifest drift** → Generate the release manifest after archives are created and checksummed in CI.
- **Release install incompatibility** → CI should smoke-test each bundle by running `bin/piccolo-gws describe --json` and `bin/piccolo-gws status --json` with an empty temp config.
