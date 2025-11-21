#!/usr/bin/env bash
set -euo pipefail

MANIFEST=${MANIFEST:-docs/mcp/manifest.json}
SECTION=${1:-commands}
FILTER=${2:-}
DRY_RUN=false

usage() {
  cat <<'DOC'
Usage: $0 [commands|datasets] [filter] [--dry-run]

Runs the commands defined in ${MANIFEST}. By default it executes every entry under "commands".
Prefix arguments with "commands" or "datasets" to run a different section, supply an optional
case-insensitive filter substring to restrict the entries, and add "--dry-run" to only print
what would run.
DOC
}

if [[ $# -gt 0 && $1 == --help ]]; then
  usage
  exit 0
fi

while [[ $# -gt 0 ]]; do
  case $1 in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    commands|datasets)
      SECTION=$1
      shift
      ;;
    --filter|-f)
      FILTER=$2
      shift 2
      ;;
    --manifest)
      MANIFEST=$2
      shift 2
      ;;
    *)
      if [[ -z $FILTER ]]; then
        FILTER=$1
        shift
      else
        break
      fi
      ;;
  esac
done

if ! [[ -r $MANIFEST ]]; then
  echo "Manifest not readable: $MANIFEST" >&2
  exit 1
fi

if [[ $SECTION != commands && $SECTION != datasets ]]; then
  echo "Unsupported section '$SECTION'; choose commands or datasets." >&2
  exit 1
fi

mapfile -t entries < <(jq -r --arg section "$SECTION" --arg filter "$FILTER" '
  .[$section]
  | map(select(
      ($filter == "") or ((.name // "") | test($filter; "i"))
    ))
  | .[] | @base64' "$MANIFEST")

if [[ ${#entries[@]} -eq 0 ]]; then
  echo "No $SECTION entries found (filter='$FILTER')." >&2
  exit 1
fi

echo "Running $SECTION entries from $MANIFEST" >&2
echo "Dry run: $DRY_RUN" >&2

for entry in "${entries[@]}"; do
  decoded=$(printf '%s' "$entry" | base64 --decode)
  name=$(printf '%s' "$decoded" | jq -r '.name // "(unnamed)"')
  command=$(printf '%s' "$decoded" | jq -r '.command // ""')
  echo "----" >&2
  echo "$name" >&2
  echo "$command" >&2
  if [[ $DRY_RUN == true ]]; then
    continue
  fi
  eval "$command"
done
