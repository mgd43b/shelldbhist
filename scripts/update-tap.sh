#!/usr/bin/env bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_OWNER="mgd43b"
REPO_NAME="shelldbhist"
TAP_REPO="homebrew-taps"
TAP_PATH="/opt/homebrew/Library/Taps/${REPO_OWNER}/${TAP_REPO}"
FORMULA_PATH="${TAP_PATH}/Formula/sdbh.rb"

# Defaults
DRY_RUN=false
SKIP_TEST=false

# Helper functions
info() {
    echo -e "${BLUE}ℹ${NC} $*"
}

success() {
    echo -e "${GREEN}✓${NC} $*"
}

warn() {
    echo -e "${YELLOW}⚠${NC} $*"
}

error() {
    echo -e "${RED}✗${NC} $*" >&2
}

usage() {
    cat <<EOF
Usage: $0 <version> [options]

Update the Homebrew tap formula for a new sdbh release.

Arguments:
  version       Version number (e.g., 0.14.0) without 'v' prefix

Options:
  --dry-run     Preview changes without committing
  --skip-test   Skip local formula testing
  -h, --help    Show this help message

Examples:
  $0 0.14.0
  $0 0.14.0 --dry-run
  $0 0.14.0 --skip-test

EOF
    exit 0
}

# Parse arguments
if [[ $# -eq 0 ]]; then
    error "No version specified"
    usage
fi

VERSION=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --skip-test)
            SKIP_TEST=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            if [[ -z "$VERSION" ]]; then
                VERSION="$1"
            else
                error "Unknown argument: $1"
                usage
            fi
            shift
            ;;
    esac
done

# Validate version format (X.Y.Z)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    error "Invalid version format: $VERSION"
    error "Expected format: X.Y.Z (e.g., 0.14.0)"
    exit 1
fi

info "Updating Homebrew tap for sdbh v${VERSION}"
if [[ "$DRY_RUN" == true ]]; then
    warn "DRY RUN MODE - No changes will be committed"
fi

# Step 1: Check if GitHub release exists
info "Checking if GitHub release v${VERSION} exists..."
TARBALL_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/archive/refs/tags/v${VERSION}.tar.gz"

if ! curl -sIf "$TARBALL_URL" > /dev/null 2>&1; then
    error "GitHub release v${VERSION} not found at:"
    error "  $TARBALL_URL"
    error ""
    error "Please ensure the release has been published first."
    exit 1
fi
success "GitHub release v${VERSION} exists"

# Step 2: Download tarball and calculate SHA256
info "Downloading tarball..."
TEMP_DIR=$(mktemp -d)
TARBALL_PATH="${TEMP_DIR}/sdbh-${VERSION}.tar.gz"

if ! curl -sL "$TARBALL_URL" -o "$TARBALL_PATH"; then
    error "Failed to download tarball"
    rm -rf "$TEMP_DIR"
    exit 1
fi
success "Downloaded tarball to ${TARBALL_PATH}"

info "Calculating SHA256 hash..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    SHA256=$(shasum -a 256 "$TARBALL_PATH" | awk '{print $1}')
else
    SHA256=$(sha256sum "$TARBALL_PATH" | awk '{print $1}')
fi
success "SHA256: ${SHA256}"

# Step 3: Update tap repository
info "Checking tap repository..."
if [[ ! -d "$TAP_PATH" ]]; then
    error "Tap repository not found at: $TAP_PATH"
    error "Please run: brew tap ${REPO_OWNER}/taps"
    rm -rf "$TEMP_DIR"
    exit 1
fi

cd "$TAP_PATH"
info "Updating tap repository..."
git fetch origin
git checkout main
git pull origin main
success "Tap repository updated"

# Step 4: Check if formula exists
if [[ ! -f "$FORMULA_PATH" ]]; then
    error "Formula not found at: $FORMULA_PATH"
    rm -rf "$TEMP_DIR"
    exit 1
fi

# Step 5: Update formula file
info "Updating Formula/sdbh.rb..."
# Create backup
cp "$FORMULA_PATH" "${FORMULA_PATH}.bak"

# Update URL line
sed -i '' "s|url \"https://github.com/${REPO_OWNER}/${REPO_NAME}/archive/refs/tags/v[0-9]*\.[0-9]*\.[0-9]*\.tar\.gz\"|url \"${TARBALL_URL}\"|" "$FORMULA_PATH"

# Update SHA256 line
sed -i '' "s|sha256 \"[a-f0-9]*\"|sha256 \"${SHA256}\"|" "$FORMULA_PATH"

# Show diff
info "Formula changes:"
git diff "$FORMULA_PATH" || true

# Step 6: Test formula (unless skipped)
if [[ "$SKIP_TEST" == false && "$DRY_RUN" == false ]]; then
    info "Testing formula locally (this may take a few minutes)..."
    warn "This will uninstall and reinstall sdbh"
    
    if brew list sdbh &> /dev/null; then
        brew uninstall sdbh || true
    fi
    
    if brew install --build-from-source sdbh; then
        INSTALLED_VERSION=$(sdbh --version | awk '{print $2}')
        if [[ "$INSTALLED_VERSION" == "$VERSION" ]]; then
            success "Formula test passed! Installed version: ${INSTALLED_VERSION}"
        else
            error "Version mismatch! Expected ${VERSION}, got ${INSTALLED_VERSION}"
            # Restore backup
            mv "${FORMULA_PATH}.bak" "$FORMULA_PATH"
            rm -rf "$TEMP_DIR"
            exit 1
        fi
    else
        error "Formula installation failed"
        # Restore backup
        mv "${FORMULA_PATH}.bak" "$FORMULA_PATH"
        rm -rf "$TEMP_DIR"
        exit 1
    fi
elif [[ "$SKIP_TEST" == true ]]; then
    warn "Skipping formula test (--skip-test)"
fi

# Step 7: Commit and push (unless dry run)
if [[ "$DRY_RUN" == true ]]; then
    warn "DRY RUN: Skipping commit and push"
    warn "Changes preview:"
    git diff "$FORMULA_PATH"
    # Restore original
    mv "${FORMULA_PATH}.bak" "$FORMULA_PATH"
else
    info "Committing changes..."
    git add "$FORMULA_PATH"
    git commit -m "chore: bump sdbh to v${VERSION}"
    
    info "Pushing to GitHub..."
    git push origin main
    
    success "Formula updated successfully!"
    success "Users can now upgrade with: brew update && brew upgrade sdbh"
    
    # Clean up backup
    rm -f "${FORMULA_PATH}.bak"
fi

# Cleanup
rm -rf "$TEMP_DIR"

info "Done!"