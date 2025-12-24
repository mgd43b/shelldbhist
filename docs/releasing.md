# Releasing sdbh

This repo ships binaries via **cargo-dist** and automates versioning/tagging via **release-please**.

## Overview
- **release-please** creates/updates a *Release PR* based on Conventional Commits.
- Merging the Release PR creates a tag `vX.Y.Z` and a GitHub Release.
- **cargo-dist** runs on tag push and uploads platform binaries to the GitHub Release.
- **Version Sync Guard** prevents version/tag mismatches during releases.

## One-time setup (repo settings)
In GitHub:
- Settings → Actions → General → Workflow permissions
  - allow GitHub Actions to create and approve pull requests

## Day-to-day release flow
1) Merge feature/fix PRs into `main` using **Conventional Commit** titles:
   - `feat: ...` (minor bump)
   - `fix: ...` (patch bump)
   - `chore: ...` (usually no release)

2) release-please will open/update a Release PR.

3) Merge the Release PR.

4) Watch GitHub Actions:
   - “Release Please” should succeed
   - “Release” (cargo-dist) should run on the new `vX.Y.Z` tag and upload binaries

## Troubleshooting

### Release exists but has no binaries
Usually means cargo-dist didn't run for the tag.
- Confirm the tag is `vX.Y.Z` (cargo-dist triggers on semver tags).
- Check Actions → workflow "Release".
- If workflow failed, check logs for "out of date contents" errors (see below).
- As fallback: manually download artifacts from failed workflow run and upload to draft release.

### cargo-dist complains release.yml is out of date
This happens when you manually modify the auto-generated workflow file.
- **Option 1**: Add `allow-dirty = true` to `[dist]` section in `sdbh/dist-workspace.toml`
- **Option 2**: Regenerate dist-managed workflows (removes manual changes):
  ```bash
  dist init -y -c github
  ```
  Commit the regenerated workflow and try again.

### Version Sync Guard workflow fails
The "Version Sync Guard" workflow ensures tag versions match source code versions.
- Check that `sdbh/Cargo.toml` version matches the tag being created.
- Check that `.release-please-manifest.json["sdbh"]` matches the tag.
- Fix version mismatches before creating tags manually.
