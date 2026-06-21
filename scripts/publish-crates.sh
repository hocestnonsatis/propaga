#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CRATES=(
    propaga-core
    propaga-domains
    propaga-engine
    propaga-propagators
    propaga-search
    propaga-model
    propaga-flatzinc
    propaga-cli
)

PUBLISH=0
ALLOW_DIRTY=()
if [[ "${1:-}" == "--publish" ]]; then
    PUBLISH=1
    shift
fi
if [[ "${1:-}" == "--allow-dirty" ]]; then
    ALLOW_DIRTY=(--allow-dirty)
fi

for crate in "${CRATES[@]}"; do
    if [[ "${PUBLISH}" -eq 0 ]]; then
        echo "--- cargo publish -p ${crate} --dry-run"
        cargo publish -p "${crate}" --dry-run "${ALLOW_DIRTY[@]}"
    else
        echo "--- cargo publish -p ${crate}"
        cargo publish -p "${crate}" "${ALLOW_DIRTY[@]}"
        sleep 30
    fi
done

if [[ "${PUBLISH}" -eq 0 ]]; then
    echo "Dry-run complete. Use --publish to upload to crates.io."
else
    echo "All crates published."
fi
