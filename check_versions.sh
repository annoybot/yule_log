#!/bin/bash
set -euo pipefail

tmpfile=$(mktemp)

# Ensure the temp file gets removed on script exit or interrupt
trap 'rm -f "$tmpfile"' EXIT

# Write cargo metadata to tmepfile.
cargo metadata --no-deps --format-version=1 > "$tmpfile"

# Extract versions from the temp file.
CORE_VERSION=$(jq -r '
  .packages[] | select(.name == "yule_log") | .version' "$tmpfile")

MACROS_VERSION=$(jq -r '
  .packages[] | select(.name == "yule_log_macros") | .version' "$tmpfile")

TESTS_VERSION=$(jq -r '
  .packages[] | select(.name == "integration_tests") | .version' "$tmpfile")

CORE_MACROS_DEP_VERSIONS=$(jq -r '
  .packages[]
  | select(.name == "yule_log")
  | .dependencies[]
  | select(.name == "yule_log_macros")
  | .req' "$tmpfile" | xargs)

echo "core         (yule_log):           $CORE_VERSION"
echo "macros       (yule_log_macros):    $MACROS_VERSION"
echo "integration  (integration_tests):  $TESTS_VERSION"
echo "core->macros dependencies:         $CORE_MACROS_DEP_VERSIONS"


# Strip leading '=' sign if any.
CORE_VERSION=$(echo "$CORE_VERSION" | sed 's/^=//')
MACROS_VERSION=$(echo "$MACROS_VERSION" | sed 's/^=//')
TESTS_VERSION=$(echo "$TESTS_VERSION" | sed 's/^=//')
CORE_MACROS_DEP_VERSIONS=$(echo "$CORE_MACROS_DEP_VERSIONS" | sed 's/=//g')

read -r -a core_macros_versions_array <<< "$CORE_MACROS_DEP_VERSIONS"
versions=("$CORE_VERSION" "$MACROS_VERSION" "$TESTS_VERSION" "${core_macros_versions_array[@]}")

echo

first="${versions[0]}"
for v in "${versions[@]:1}"; do
  if [[ "$v" != "$first" ]]; then
    echo "Version mismatch detected!"
    echo "Versions found: ${versions[*]}"
    exit 1
  fi
done

echo "All versions match: $first"
