#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
data_dir=${1:-"$repo_dir"}
executable=${2:-"$repo_dir/target/release/comanche3-mvp"}

if (( $# > 2 )); then
    printf 'Usage: %s [game-data-directory] [release-executable]\n' "$0" >&2
    exit 2
fi
if [[ ! -r "$data_dir/RESOURCE.RES" ]]; then
    printf 'Missing or unreadable RESOURCE.RES in %s\n' "$data_dir" >&2
    exit 2
fi
if [[ ! -f "$executable" || ! -x "$executable" ]]; then
    printf 'Missing executable at %s. Build with cargo build --release --locked first.\n' "$executable" >&2
    exit 2
fi
executable="$(cd -- "$(dirname -- "$executable")" && pwd -P)/$(basename -- "$executable")"
cd -- "$data_dir"

fail() {
    printf '%s\n' "$1" >&2
    printf '%s\n' "$output" >&2
    exit 1
}

verify_route() {
    local resolution=$1
    shift
    output=$("$executable" --smoke "$@" 2>&1) || fail "Route smoke test failed at $resolution."
    local expected_progress
    expected_progress=$(printf 'Ponto %s/4 confirmado\n' 1 2 3 4)
    [[ $(sed -n '/^Ponto [0-9]/p' <<< "$output") == "$expected_progress" ]] ||
        fail "Unexpected objective order at $resolution."
    grep -Fq "Treinamento concluido; $resolution;" <<< "$output" ||
        fail "Missing mission completion at $resolution."
    printf '%s\n' "$output"
}

verify_route 1920x1080 --1080p
verify_route 3840x2160 --4k --training "$repo_dir/missions/turns.route"

output=$("$executable" --check-missions 2>&1) || fail 'Mission corpus audit failed.'
grep -Fxq 'campaign-named: 41 files, 2124 rules, 1 empty blocks' <<< "$output" ||
    fail 'Unexpected campaign corpus counts.'
grep -Fxq 'multiplayer: 5 files, 26 rules, 0 empty blocks' <<< "$output" ||
    fail 'Unexpected multiplayer corpus counts.'
printf '%s\n' "$output"

invalid_route=$(mktemp)
trap 'unlink -- "$invalid_route"' EXIT
cat > "$invalid_route" <<'ROUTE'
point, NaN, 60
; Trigger Data BEGINS
if
reached, 1
then
advance, 1
<
; Trigger Data ENDS
ROUTE
if output=$("$executable" --smoke --training "$invalid_route" 2>&1); then
    fail 'The malformed route was accepted.'
fi
grep -Fq 'invalid finite coordinate' <<< "$output" ||
    fail 'Malformed route failed for an unexpected reason.'
printf 'Malformed route rejected.\n'
