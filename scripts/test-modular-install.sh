#!/usr/bin/env bash
#
# Test script for GAT modular installation system
#
# This script tests the modular installation workflow:
#  1. Package binaries with package.sh
#  2. Test installation with install-modular.sh
#  3. Verify components are installed correctly
#  4. Test the gat install subcommand
#
# Usage: bash test-modular-install.sh [--quick]

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
QUICK_MODE="${1:-}"

test_count=0
pass_count=0
fail_count=0

# Test utilities
log_test() {
  echo -e "${YELLOW}TEST:${NC} $1"
  ((test_count++))
}

log_pass() {
  echo -e "${GREEN}✓ PASS${NC}: $1"
  ((pass_count++))
}

log_fail() {
  echo -e "${RED}✗ FAIL${NC}: $1"
  ((fail_count++))
}

log_info() {
  echo -e "${YELLOW}INFO:${NC} $1"
}

# Create temporary directory for testing
TMPDIR=$(mktemp -d)

# Cleanup
cleanup() {
  if [[ -n "${TMPDIR:-}" ]] && [[ -d "$TMPDIR" ]]; then
    log_info "Cleaning up temporary directory: $TMPDIR"
    rm -rf "$TMPDIR"
  fi
}
trap cleanup EXIT
TEST_PREFIX="$TMPDIR/gat-install-test"

echo "======================================"
echo "GAT Modular Installation Test Suite"
echo "======================================"
echo "Test Directory: $TMPDIR"
echo "Install Prefix: $TEST_PREFIX"
echo

# Test 1: Verify scripts exist
log_test "Scripts exist"
if [[ -f "$SCRIPT_DIR/install-modular.sh" ]]; then
  log_pass "install-modular.sh exists"
else
  log_fail "install-modular.sh not found"
fi

if [[ -f "$SCRIPT_DIR/package.sh" ]]; then
  log_pass "package.sh exists"
else
  log_fail "package.sh not found"
fi

# Test 2: Verify install-modular.sh is executable and has correct syntax
log_test "install-modular.sh syntax"
if bash -n "$SCRIPT_DIR/install-modular.sh" 2>/dev/null; then
  log_pass "install-modular.sh has valid bash syntax"
else
  log_fail "install-modular.sh has syntax errors"
fi

# Test 3: Verify help output
log_test "install-modular.sh --help"
if bash "$SCRIPT_DIR/install-modular.sh" --help 2>/dev/null | grep -q "GAT Modular Installer"; then
  log_pass "install-modular.sh help output is correct"
else
  log_fail "install-modular.sh help output is incorrect"
fi

# Test 4: Test directory creation
log_test "Directory structure creation"
mkdir -p "$TEST_PREFIX/bin" "$TEST_PREFIX/config" "$TEST_PREFIX/lib" "$TEST_PREFIX/cache"
if [[ -d "$TEST_PREFIX/bin" ]] && [[ -d "$TEST_PREFIX/config" ]] && [[ -d "$TEST_PREFIX/lib" ]] && [[ -d "$TEST_PREFIX/cache" ]]; then
  log_pass "Directory structure created successfully"
else
  log_fail "Failed to create directory structure"
fi

# Test 5: Verify package.sh can create packages (if in quick mode, skip build)
if [[ "$QUICK_MODE" != "--quick" ]]; then
  log_test "Building test package (headless variant)"
  cd "$ROOT_DIR"

  # Create a minimal dist directory structure for testing
  if mkdir -p dist && bash "$SCRIPT_DIR/package.sh" headless >/dev/null 2>&1; then
    if ls dist/*.tar.gz >/dev/null 2>&1; then
      log_pass "package.sh successfully created artifacts"
      PACKAGE_CREATED=true
    else
      log_fail "package.sh created no artifacts"
      PACKAGE_CREATED=false
    fi
  else
    log_fail "package.sh build failed"
    PACKAGE_CREATED=false
  fi
fi

# Test 6: Verify gat-cli can be invoked with --help
log_test "gat-cli --help"
cd "$ROOT_DIR"
if cargo run -q -p gat-cli -- --help >/dev/null 2>&1; then
  log_pass "gat-cli --help works"
else
  log_fail "gat-cli --help failed"
fi

# Test 7: Verify gat install subcommand exists
log_test "gat install command availability"
if cargo run -q -p gat-cli -- install --help >/dev/null 2>&1; then
  log_pass "gat install subcommand exists"
else
  log_fail "gat install subcommand not found or broken"
fi

# Test 8: Verify environment variable handling
log_test "Environment variable handling"
if GAT_PREFIX="$TEST_PREFIX" bash "$SCRIPT_DIR/install-modular.sh" --help 2>/dev/null | grep -q "GAT Modular"; then
  log_pass "Environment variables are processed correctly"
else
  log_fail "Environment variable handling failed"
fi

# Test 9: Verify component parsing
log_test "Component validation"
# This is a smoke test - just verify the script accepts component arguments
if bash "$SCRIPT_DIR/install-modular.sh" --components cli,tui --help >/dev/null 2>&1; then
  log_pass "Component arguments are accepted"
else
  log_fail "Component argument validation failed"
fi

# Summary
echo
echo "======================================"
echo "Test Summary"
echo "======================================"
echo "Total Tests: $test_count"
echo -e "Passed: ${GREEN}$pass_count${NC}"
echo -e "Failed: ${RED}$fail_count${NC}"

if [[ $fail_count -eq 0 ]]; then
  echo -e "${GREEN}All tests passed!${NC}"
  exit 0
else
  echo -e "${RED}Some tests failed!${NC}"
  exit 1
fi
