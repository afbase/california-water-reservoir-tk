#!/usr/bin/env bash
# Sourced by build.yml. Sets BASELINE to the observations CSV that incremental
# fetches should resume from: the copy cached by the previous run when it is at
# least as large as the committed fixture, otherwise the committed fixture.
# (The cache can be missing — GitHub evicts caches unused for 7 days — or stale
# after the committed baseline is refreshed.)

BASELINE=fixtures/observations.csv
_cached=fixtures/observations.cache.csv

if [ -s "$_cached" ] && [ "$(wc -c < "$_cached")" -ge "$(wc -c < "$BASELINE")" ]; then
  BASELINE="$_cached"
fi

echo "Baseline observations: ${BASELINE} ($(wc -c < "$BASELINE") bytes)"
export BASELINE
