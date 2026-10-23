#!/usr/bin/env bash
# Usage: bench-compare.sh <criterion dir> <threshold, e.g. 0.10>
# Reads criterion's change estimates and fails if a mean slowed down beyond the threshold.
set -euo pipefail
dir=${1:-target/criterion}
threshold=${2:-0.10}
status=0
{
  echo "| Benchmark | Change |"
  echo "|---|---|"
} >> "${GITHUB_STEP_SUMMARY:-/dev/null}"
while IFS= read -r file; do
  name=${file#"$dir"/}
  name=${name%/change/estimates.json}
  change=$(jq '.mean.point_estimate' "$file")
  percent=$(awk -v c="$change" 'BEGIN { printf "%+.1f %%", c * 100 }')
  echo "| $name | $percent |" >> "${GITHUB_STEP_SUMMARY:-/dev/null}"
  if awk -v c="$change" -v t="$threshold" 'BEGIN { exit !(c > t) }'; then
    echo "::error title=Benchmark regression::$name is $percent slower than the last tag"
    status=1
  else
    echo "$name: $percent"
  fi
done < <(find "$dir" -path '*/change/estimates.json' | sort)
exit $status
