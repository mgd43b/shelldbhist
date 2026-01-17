# Releasing sdbh

This repo ships binaries via **cargo-dist** and automates versioning/tagging via **release-please**.

## Overview
1. **release-please** creates/updates a *Release PR* based on Conventional Commits.
2. Merging the Release PR creates a tag `vX.Y.Z` and a GitHub Release.
3. The tag push automatically triggers **cargo-dist** to build and upload platform binaries.
4. **Version Sync Guard** prevents version/tag mismatches during releases.

## One-time setup (repo settings)
In GitHub:
- Settings → Actions → General → Workflow permissions
  - allow GitHub Actions to create and approve pull requests

## Day-to-day release flow
1) Merge feature/fix PRs into `main` using **Conventional Commit** titles:
   - `feat: ...` (minor bump: 0.12.0 → 0.13.0)
   - `fix: ...` (patch bump: 0.12.0 → 0.12.1)
   - `chore: ...` (usually no release)

2) release-please will open/update a Release PR.

3) Merge the Release PR.

4) Watch GitHub Actions:
   - “Release Please” should succeed
   - “Release” (cargo-dist) should run on the new `vX.Y.Z` tag and upload binaries

## Troubleshooting

### Release exists but has no binaries
This usually means the cargo-dist workflow didn't run or failed for the tag.

**Common causes:**
- Tag format doesn't match `vX.Y.Z` pattern (cargo-dist only triggers on semver tags)
- Workflow failed during build (check logs)
- Version Sync Guard failed (see below)

**Steps to diagnose:**
1. Check Actions → "Release" workflow for the tag
2. If workflow didn't run at all, verify the tag format matches `v[0-9]+.[0-9]+.[0-9]+`
3. If workflow failed, check logs for specific errors
4. **Manual fix**: Download artifacts from the failed workflow run and upload to the release manually

### Manually triggering a release build
If a tag exists but the workflow didn't run or failed:
1. Go to Actions → "Release" workflow
2. Click "Run workflow"
3. Enter the tag (e.g., `v0.13.0`)
4. Click "Run workflow"

This will re-run cargo-dist for that tag and upload the binaries.

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

## Recent Changes (January 2026)

**Workflow reliability improvements:**
- Removed the `workflow_run` trigger that caused race conditions
- The Release workflow now triggers reliably on tag push events only
- Simplified tag resolution logic (removed 100+ lines of complex bash)
- Added clearer error messages and debugging output

**Why this matters:**
The previous setup tried to trigger cargo-dist immediately after release-please completed, but this created a race condition where the tag might not be visible yet via the GitHub API. This caused intermittent failures where releases would be created but have no binary assets attached. The new approach is simpler and more reliable - it waits for the tag to actually exist before building.