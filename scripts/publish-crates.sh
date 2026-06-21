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

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

is_published() {
    local crate=$1
    cargo search "${crate}" --limit 1 2>/dev/null | grep -Fq "${crate} = \"${VERSION}\""
}

already_on_registry() {
    local output=$1
    grep -Eiq 'already exists|already uploaded|duplicate version|cannot publish.*already' <<<"${output}"
}

publish_crate() {
    local crate=$1
    local attempt output status

    for attempt in 1 2 3 4 5; do
        set +e
        output=$(cargo publish -p "${crate}" "${ALLOW_DIRTY[@]}" 2>&1)
        status=$?
        set -e

        if [[ "${status}" -eq 0 ]]; then
            echo "${output}"
            return 0
        fi

        echo "${output}" >&2

        if already_on_registry "${output}"; then
            echo "Skipping ${crate} ${VERSION}: already on crates.io"
            return 0
        fi

        if grep -Fq "429 Too Many Requests" <<<"${output}" && [[ "${attempt}" -lt 5 ]]; then
            echo "Rate limited publishing ${crate}; waiting 90s (attempt ${attempt}/5)..."
            sleep 90
            continue
        fi

        return "${status}"
    done
}

FAILED=()
for crate in "${CRATES[@]}"; do
    if [[ "${PUBLISH}" -eq 0 ]]; then
        echo "--- cargo publish -p ${crate} --dry-run"
        cargo publish -p "${crate}" --dry-run "${ALLOW_DIRTY[@]}"
        continue
    fi

    echo "--- ${crate}"
    if is_published "${crate}"; then
        echo "Skipping ${crate} ${VERSION}: already on crates.io"
        continue
    fi

    if publish_crate "${crate}"; then
        echo "Published ${crate} ${VERSION}"
        sleep 90
    else
        FAILED+=("${crate}")
        echo "Failed to publish ${crate}; continuing with remaining crates."
    fi
done

if [[ "${PUBLISH}" -eq 0 ]]; then
    echo "Dry-run complete. Use --publish to upload to crates.io."
elif ((${#FAILED[@]} == 0)); then
    echo "All crates published or already present on crates.io."
else
    echo "Failed crates: ${FAILED[*]}"
    echo "Re-run this script later to retry failed crates."
    exit 1
fi
