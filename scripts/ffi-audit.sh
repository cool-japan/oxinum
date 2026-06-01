#!/usr/bin/env bash
set -euo pipefail
RESULT=$(cargo tree --edges normal 2>/dev/null | grep -E '(rug v|gmp-mpfr-sys v|gmp-sys v|mpfr-sys v|gmp-mpfr v)' || true)
if [ -n "$RESULT" ]; then
  echo "FFI AUDIT FAILED: forbidden crates found in dependency tree:"
  echo "$RESULT"
  exit 1
fi
echo "FFI audit PASSED"
