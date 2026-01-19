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

## Updating the Homebrew Tap

After releasing a new version, you need to update the Homebrew tap formula so users can install the latest version via Homebrew.

### Automated Method (Recommended)

Use the `scripts/update-tap.sh` script to automate the entire process:

```bash
# Update to the latest version (e.g., 0.14.0)
./scripts/update-tap.sh 0.14.0

# Preview changes without committing (dry run)
./scripts/update-tap.sh 0.14.0 --dry-run

# Skip local testing (faster, but less safe)
./scripts/update-tap.sh 0.14.0 --skip-test
```

**What the script does:**
1. Validates the version format (X.Y.Z)
2. Checks that the GitHub release exists
3. Downloads the tarball and calculates SHA256 hash
4. Updates the tap repository's Formula/sdbh.rb
5. Tests the formula locally (unless `--skip-test`)
6. Commits and pushes to `homebrew-taps` repository

**Requirements:**
- The GitHub release must be published first (via release-please workflow)
- You must have the tap installed locally: `brew tap mgd43b/taps`
- The tap repository must be accessible at `/opt/homebrew/Library/Taps/mgd43b/homebrew-taps`

### Manual Method

If you prefer to update the formula manually:

```bash
# 1. Get the new version's tarball URL and download it
VERSION="0.14.0"
URL="https://github.com/mgd43b/shelldbhist/archive/refs/tags/v${VERSION}.tar.gz"
curl -L "$URL" -o "/tmp/sdbh-${VERSION}.tar.gz"

# 2. Calculate the SHA256 hash
shasum -a 256 "/tmp/sdbh-${VERSION}.tar.gz"

# 3. Update Formula/sdbh.rb in the tap repository
cd /opt/homebrew/Library/Taps/mgd43b/homebrew-taps
# Edit Formula/sdbh.rb:
#   - Update url to point to the new version
#   - Update sha256 with the calculated hash

# 4. Test locally
brew uninstall sdbh
brew install --build-from-source sdbh
sdbh --version  # Should show the new version

# 5. Commit and push
git add Formula/sdbh.rb
git commit -m "chore: bump sdbh to v${VERSION}"
git push origin main
```

## Recent Changes (January 2026)

**Workflow reliability improvements:**
- Removed the `workflow_run` trigger that caused race conditions
- The Release workflow now triggers reliably on tag push events only
- Simplified tag resolution logic (removed 100+ lines of complex bash)
- Added clearer error messages and debugging output

**Homebrew tap automation:**
- Added `scripts/update-tap.sh` to automate formula updates
- Created GitHub repository at `mgd43b/homebrew-taps`
- Fixed missing git remote and duplicate tap conflicts

**Why this matters:**
The previous setup tried to trigger cargo-dist immediately after release-please completed, but this created a race condition where the tag might not be visible yet via the GitHub API. This caused intermittent failures where releases would be created but have no binary assets attached. The new approach is simpler and more reliable - it waits for the tag to actually exist before building.