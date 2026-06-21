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

publish_crate() {
    local crate=$1
    local attempt output
    for attempt in 1 2 3 4 5; do
        set +e
        output=$(cargo publish -p "${crate}" "${ALLOW_DIRTY[@]}" 2>&1)
        local status=$?
        set -e
        if [[ "${status}" -eq 0 ]]; then
            echo "${output}"
            return 0
        fi
        echo "${output}" >&2
        if echo "${output}" | rg -q "already (exists|uploaded)|duplicate version"; then
            echo "Skipping ${crate}: version already on crates.io"
            return 0
        fi
        if echo "${output}" | rg -q "429 Too Many Requests" && [[ "${attempt}" -lt 5 ]]; then
            echo "Rate limited publishing ${crate}; waiting 90s (attempt ${attempt}/5)..."
            sleep 90
            continue
        fi
        return "${status}"
    done
}

for crate in "${CRATES[@]}"; do
    if [[ "${PUBLISH}" -eq 0 ]]; then
        echo "--- cargo publish -p ${crate} --dry-run"
        cargo publish -p "${crate}" --dry-run "${ALLOW_DIRTY[@]}"
    else
        echo "--- cargo publish -p ${crate}"
        if ! publish_crate "${crate}"; then
            echo "Stopped at ${crate}. Re-run this script later to publish remaining crates."
            exit 1
        fi
        sleep 90
    fi
done

if [[ "${PUBLISH}" -eq 0 ]]; then
    echo "Dry-run complete. Use --publish to upload to crates.io."
else
    echo "All crates published."
fi
