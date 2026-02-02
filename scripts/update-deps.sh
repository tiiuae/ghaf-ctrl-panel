#!/usr/bin/env bash
# Update all package dependencies for ghaf-ctrl-panel
#
# This script updates:
# - Nix flake inputs (flake.lock)
# - Cargo dependencies (Cargo.lock)
#
# Usage: ./scripts/update-deps.sh [OPTIONS]
#
# Options:
#   --all       Update all dependencies (default)
#   --nix       Update only Nix flake inputs
#   --cargo     Update only Cargo dependencies
#   --upgrade   Upgrade source dependencies (Cargo.toml) to latest versions
#   --help      Show this help message
#
# Examples:
#   ./scripts/update-deps.sh              # Update all lock files
#   ./scripts/update-deps.sh --upgrade    # Upgrade source dependencies + update lock files
#   ./scripts/update-deps.sh --cargo      # Update only Cargo.lock
#   ./scripts/update-deps.sh --cargo --upgrade  # Upgrade Cargo.toml versions + update lock

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
info() {
  echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
  echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the project root
if [[ ! -f "flake.nix" ]]; then
  error "Must be run from the project root directory"
  exit 1
fi

update_nix() {
  info "Updating Nix flake inputs..."

  if ! command -v nix &> /dev/null; then
    error "Nix is not installed"
    return 1
  fi

  # Update all flake inputs
  nix flake update

  success "Nix flake inputs updated"

  # Show what changed
  info "Flake input changes:"
  git --no-pager diff flake.lock | grep -E "^\+.*\"(narHash|rev)\"" | head -20 || true
}

update_cargo() {
  local upgrade_mode=${1:-false}

  if [[ $upgrade_mode == true ]]; then
    info "Upgrading Cargo dependencies (will update Cargo.toml versions)..."
  else
    info "Updating Cargo dependencies (lock files only)..."
  fi

  if ! command -v cargo &> /dev/null; then
    warn "Cargo not found in PATH, trying via nix develop..."
    if ! nix develop -c cargo --version &> /dev/null; then
      error "Cargo is not available"
      return 1
    fi
    CARGO_CMD="nix develop -c cargo"
  else
    CARGO_CMD="cargo"
  fi

  # If upgrade mode, use cargo-upgrade if available
  if [[ $upgrade_mode == true ]]; then
    if $CARGO_CMD upgrade --help &> /dev/null; then
      warn "⚠️  UPGRADE MODE: This will modify Cargo.toml files!"

      info "Running cargo upgrade in project root..."
      $CARGO_CMD upgrade || warn "cargo upgrade failed"

      success "Cargo.toml files upgraded"
    else
      warn "cargo-upgrade not available"
      warn "To enable version upgrades, install cargo-edit:"
      warn "  cargo install cargo-edit"
      warn "Falling back to lock file updates only"
    fi
  fi

  # Update workspace dependencies
  info "Updating Cargo.lock..."
  $CARGO_CMD update

  success "Cargo dependencies updated"

  # Show major version changes
  info "Checking for major version changes..."
  git --no-pager diff Cargo.lock 2> /dev/null | grep -E "^[\+\-]version = " | head -20 || true
}

verify_updates() {
  info "Verifying updates..."

  # Check if nix build still works
  if command -v nix &> /dev/null; then
    info "Testing Nix build..."
    if nix build 2>&1 | grep -q "error:"; then
      warn "Nix build reported errors"
    else
      success "Nix build verification passed"
    fi
  fi

  # Run flake checks
  if command -v nix &> /dev/null; then
    info "Running flake checks..."
    if nix flake check 2>&1 | grep -q "error:"; then
      warn "Flake check reported errors"
    else
      success "Flake check passed"
    fi
  fi
}

show_summary() {
  echo ""
  info "Update Summary:"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  # Show git status
  if git diff --quiet; then
    info "No changes detected"
  else
    info "Changed files:"
    git status --short | grep -E "^\s*M\s+" | awk '{print "  - " $2}'
  fi

  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo ""
  info "Next steps:"
  echo "  1. Review changes: git diff"
  echo "  2. Test build: nix build"
  echo "  3. Run checks: nix flake check"
  echo "  4. Commit: git add -A && git commit -sm 'chore: update dependencies'"
}

main() {
  local update_all=true
  local update_nix_only=false
  local update_cargo_only=false
  local upgrade_mode=false

  # Parse arguments
  for arg in "$@"; do
    case $arg in
    --all)
      update_all=true
      ;;
    --nix)
      update_all=false
      update_nix_only=true
      ;;
    --cargo)
      update_all=false
      update_cargo_only=true
      ;;
    --upgrade)
      upgrade_mode=true
      ;;
    --help | -h)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Update ghaf-ctrl-panel dependencies"
      echo ""
      echo "Options:"
      echo "  --all       Update all dependencies (default)"
      echo "  --nix       Update only Nix flake inputs"
      echo "  --cargo     Update only Cargo dependencies"
      echo "  --upgrade   Upgrade source dependencies (Cargo.toml)"
      echo "              to latest compatible versions (potentially breaking)"
      echo "  --help      Show this help message"
      echo ""
      echo "Examples:"
      echo "  $0                    # Update all lock files"
      echo "  $0 --upgrade          # Upgrade source dependencies + update locks"
      echo "  $0 --cargo            # Update only Cargo.lock"
      echo "  $0 --cargo --upgrade  # Upgrade Cargo.toml + update Cargo.lock"
      echo ""
      echo "Notes:"
      echo "  - Without --upgrade: Only updates lock files (safe, no breaking changes)"
      echo "  - With --upgrade: Updates version constraints in source files"
      echo "    (may introduce breaking changes, requires testing)"
      exit 0
      ;;
    *)
      error "Unknown option: $arg"
      echo "Use --help for usage information"
      exit 1
      ;;
    esac
  done

  echo ""
  echo "╔════════════════════════════════════════════════════╗"
  echo "║      ghaf-ctrl-panel Dependency Update Script      ║"
  echo "╚════════════════════════════════════════════════════╝"
  echo ""

  if [[ $upgrade_mode == true ]]; then
    warn "⚠️  UPGRADE MODE ENABLED"
    warn "This will modify source files (Cargo.toml)"
    warn "and may introduce breaking changes!"
    warn "Please review all changes and test thoroughly before committing."
    echo ""
  fi

  # Execute updates based on flags
  if [[ $update_all == true ]]; then
    update_nix || warn "Nix update failed"
    echo ""
    update_cargo "$upgrade_mode" || warn "Cargo update failed"
  else
    [[ $update_nix_only == true ]] && update_nix
    [[ $update_cargo_only == true ]] && update_cargo "$upgrade_mode"
  fi

  echo ""
  verify_updates

  echo ""
  show_summary

  if [[ $upgrade_mode == true ]]; then
    echo ""
    warn "⚠️  REMINDER: UPGRADE MODE was used"
    warn "Please carefully review ALL changes and run full test suite!"
  fi
}

# Run main function
main "$@"
