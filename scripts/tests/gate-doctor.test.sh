#!/usr/bin/env bash
# Tests for scripts/gate-doctor.py, against a fake `gh` on PATH that answers
# each API path with canned JSON or a canned HTTP error.
#
# What matters: every prerequisite is reported ok, missing or unknown; a
# missing one names its exact fix; a setting the caller cannot read is unknown
# with the permission it needs, never present or missing; the doctor never
# calls a write method; its output is the same on every run; a bad argument
# is a usage error.
set -euo pipefail
export LC_ALL=C

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
doctor="$here/../gate-doctor.py"
labels="$here/../../.github/labels.json"
workflow="$here/../../.github/workflows/merge-gate.yml"
pass=0
fail=0

ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; [[ -n ${2:-} ]] && while IFS= read -r l; do echo "     | $l"; done <<<"$2"; return 0; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin"
export FAKE_GH="$tmp/canned"
writes="$tmp/writes"
: >"$writes"
export FAKE_GH_WRITES="$writes"

# The fake gh: `gh api [--paginate] PATH` prints $FAKE_GH/<key>.json, or prints
# $FAKE_GH/<key>.err to stderr and exits 1, where <key> is PATH with every
# character outside [A-Za-z0-9] turned into `_`. Any write is recorded and refused.
cat >"$tmp/bin/gh" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
refuse() { echo "gh $*" >>"$FAKE_GH_WRITES"; echo "fake gh: refused a write: $*" >&2; exit 1; }
if [[ ${1:-} == repo && ${2:-} == view ]]; then
  [[ -f $FAKE_GH/repo_view.json ]] || { echo "fake gh: no repository here" >&2; exit 1; }
  cat "$FAKE_GH/repo_view.json"
  exit 0
fi
[[ ${1:-} == api ]] || { echo "fake gh: unexpected command: $*" >&2; exit 1; }
shift
path=
for ((i = 1; i <= $#; i++)); do
  arg=${!i}
  case $arg in
    --method | --method=* | --input | --input=* | -f | -F | --field | --field=* | --raw-field | --raw-field=* | -f* | -F*) refuse api "$@" ;;
    -X | --request) j=$((i + 1)); [[ ${!j:-} == GET ]] || refuse api "$@"; i=$j ;;
    -X*) [[ $arg == -XGET ]] || refuse api "$@" ;;
    --paginate) ;;
    -*) echo "fake gh: unexpected flag $arg" >&2; exit 1 ;;
    *) path=$arg ;;
  esac
done
key=${path//[^A-Za-z0-9]/_}
if [[ -f $FAKE_GH/$key.err ]]; then
  echo '{"message": "canned error"}'
  cat "$FAKE_GH/$key.err" >&2
  exit 1
fi
[[ -f $FAKE_GH/$key.json ]] || { echo "fake gh: no canned response for $path" >&2; exit 1; }
cat "$FAKE_GH/$key.json"
FAKE
chmod +x "$tmp/bin/gh"
export PATH="$tmp/bin:$PATH"

repo=openplush/openplush
api="repos/$repo"
keyof() { local p=$1; echo "${p//[^A-Za-z0-9]/_}"; }
# can <path> <json>: the fake answers <path> with <json>.
can() { rm -f "$FAKE_GH/$(keyof "$1").err"; printf '%s\n' "$2" >"$FAKE_GH/$(keyof "$1").json"; }
# refuse <path> <status> <message>: the fake answers <path> with an HTTP error.
refuse() { rm -f "$FAKE_GH/$(keyof "$1").json"; printf 'gh: %s (HTTP %s)\n' "$3" "$2" >"$FAKE_GH/$(keyof "$1").err"; }

# A repository with every prerequisite in place: each case changes one thing.
all_present() {
  rm -rf "$FAKE_GH"
  mkdir -p "$FAKE_GH"
  can "$api/labels?per_page=100" "$(jq -c '[.[] | {name, color, description}]' "$labels")"
  can "$api" '{"full_name": "openplush/openplush", "allow_auto_merge": true, "permissions": {"admin": true, "push": true, "pull": true}}'
  can "$api/actions/secrets/MERGE_GATE_APP_ID" '{"name": "MERGE_GATE_APP_ID", "created_at": "2026-10-01T00:00:00Z"}'
  can "$api/actions/secrets/MERGE_GATE_PRIVATE_KEY" '{"name": "MERGE_GATE_PRIVATE_KEY", "created_at": "2026-10-01T00:00:00Z"}'
  can "$api/actions/variables/MERGE_GATE_MODE" '{"name": "MERGE_GATE_MODE", "value": "on"}'
  can repo-view '{"nameWithOwner": "openplush/openplush"}'
}

# run [args...]: the doctor's exit status in $status and its output in $out.
run() { status=0; out=$(python3 "$doctor" "$@" 2>&1) || status=$?; }
# expect <case> <status> <pattern>...: the exit status, and a line matching each extended regex.
expect() {
  local name=$1 want=$2 p
  shift 2
  if [[ $status != "$want" ]]; then bad "$name (exit $status, want $want)" "$out"; return; fi
  for p in "$@"; do
    grep -qE -- "$p" <<<"$out" || { bad "$name: no line matches $p" "$out"; return; }
  done
  ok "$name"
}

all_present
run --repo "$repo"
expect "everything present passes" 0 '^ok +label tier:H$' '^ok +auto-merge' '^ok +secret MERGE_GATE_APP_ID' '^ok +variable MERGE_GATE_MODE'
if grep -qE '^(missing|unknown) ' <<<"$out"; then bad "everything present reports nothing missing or unknown" "$out"; else ok "everything present reports nothing missing or unknown"; fi
want=$(jq length "$labels")
got=$(grep -cE '^ok +label ' <<<"$out" || true)
if [[ $got == "$want" ]]; then ok "one line per label in labels.json ($want)"; else bad "expected $want label lines, got $got" "$out"; fi
{ grep -E '^[a-z]+ +label ' <<<"$out" || true; } | sed -E 's/^[a-z]+ +label //' >"$tmp/order"
if [[ -s $tmp/order ]] && sort -c "$tmp/order" 2>/dev/null; then ok "labels are listed in sorted order"; else bad "labels are listed in sorted order" "$(cat "$tmp/order")"; fi
first=$out
run --repo "$repo"
if [[ $out == "$first" ]]; then ok "two runs print the same output"; else bad "two runs print the same output" "$(diff <(echo "$first") <(echo "$out") || true)"; fi
for name in $(grep -oE '(secrets|vars)\.[A-Z_]+' "$workflow" | grep -v GITHUB_TOKEN | sed -E 's/^(secrets|vars)\.//' | sort -u); do
  if grep -qE "^ok +(secret|variable) $name( |:|$)" <<<"$out"; then ok "checks $name, which merge-gate.yml reads"; else bad "checks $name, which merge-gate.yml reads" "$out"; fi
done

all_present
can "$api/labels?per_page=100" "$(jq -c '[.[] | select(.name != "tier:H") | {name}]' "$labels")"
run --repo "$repo"
expect "a missing label is named with its fix" 1 '^missing +label tier:H$' "gh label create tier:H --repo $repo --color b60205 --description \"The maintainer merges"

all_present
can "$api/labels?per_page=100" "$(jq -c '[.[] | {name}] | .[:5]' "$labels")
$(jq -c '[.[] | {name}] | .[5:]' "$labels")"
run --repo "$repo"
expect "labels over two pages are all read" 0 '^ok +label tier:H$'

all_present
can "$api" '{"full_name": "openplush/openplush", "allow_auto_merge": false, "permissions": {"admin": true}}'
run --repo "$repo"
expect "auto-merge off is named with its fix" 1 '^missing +auto-merge' "gh repo edit $repo --enable-auto-merge" "https://github.com/$repo/settings"

all_present
can "$api" '{"full_name": "openplush/openplush", "permissions": {"admin": false, "push": true}}'
run --repo "$repo"
expect "auto-merge hidden from a non-admin is unknown with the permission" 1 '^unknown +auto-merge.*admin'

all_present
refuse "$api/actions/variables/MERGE_GATE_MODE" 404 "Not Found"
run --repo "$repo"
expect "MERGE_GATE_MODE unset is missing, with what turns the gate on" 1 '^missing +variable MERGE_GATE_MODE: unset' "gh variable set MERGE_GATE_MODE --repo $repo --body dry-run"

all_present
can "$api/actions/variables/MERGE_GATE_MODE" '{"name": "MERGE_GATE_MODE", "value": "off"}'
run --repo "$repo"
expect "MERGE_GATE_MODE off is missing, with what turns the gate on" 1 '^missing +variable MERGE_GATE_MODE: off' "gh variable set MERGE_GATE_MODE --repo $repo --body dry-run"

all_present
can "$api/actions/variables/MERGE_GATE_MODE" '{"name": "MERGE_GATE_MODE", "value": "yes"}'
run --repo "$repo"
expect "an unrecognised MERGE_GATE_MODE is missing" 1 "^missing +variable MERGE_GATE_MODE: 'yes'"

all_present
can "$api/actions/variables/MERGE_GATE_MODE" '{"name": "MERGE_GATE_MODE", "value": "dry-run"}'
run --repo "$repo"
expect "MERGE_GATE_MODE dry-run passes and names what makes it live" 0 '^ok +variable MERGE_GATE_MODE: dry-run' "gh variable set MERGE_GATE_MODE --repo $repo --body on"

all_present
refuse "$api/actions/secrets/MERGE_GATE_PRIVATE_KEY" 404 "Not Found"
run --repo "$repo"
expect "a missing App secret is named with its fix" 1 '^missing +secret MERGE_GATE_PRIVATE_KEY' "gh secret set MERGE_GATE_PRIVATE_KEY --repo $repo" "https://github.com/$repo/settings/secrets/actions"

all_present
refuse "$api/actions/secrets/MERGE_GATE_APP_ID" 403 "Must have admin rights to Repository."
refuse "$api/actions/variables/MERGE_GATE_MODE" 403 "Resource not accessible by personal access token"
run --repo "$repo"
expect "a 403 is unknown, naming the permission" 1 '^unknown +secret MERGE_GATE_APP_ID: .*admin.*Secrets' '^unknown +variable MERGE_GATE_MODE: .*admin.*Variables'

all_present
can "$api" '{"full_name": "openplush/openplush", "allow_auto_merge": true, "permissions": {"admin": false, "push": true}}'
refuse "$api/actions/secrets/MERGE_GATE_APP_ID" 404 "Not Found"
run --repo "$repo"
expect "a 404 a non-admin may be shown for a hidden secret is unknown, not missing" 1 '^unknown +secret MERGE_GATE_APP_ID: .*admin'

all_present
refuse "$api/labels?per_page=100" 403 "Resource not accessible"
run --repo "$repo"
expect "unreadable labels are each unknown" 1 '^unknown +label tier:A: ' '^unknown +label tier:H: '

all_present
refuse "$api/actions/variables/MERGE_GATE_MODE" 502 "Bad Gateway"
run --repo "$repo"
expect "a server error is unknown and quotes gh" 1 '^unknown +variable MERGE_GATE_MODE: .*Bad Gateway'

all_present
run
expect "the repository defaults to the one gh sees" 0 "^gate-doctor: $repo\$"

all_present
rm "$FAKE_GH/repo_view.json"
run
expect "no repository and no --repo is a usage error" 2 '--repo'

all_present
run --bogus
expect "an unknown argument is a usage error" 2 'usage'
run --repo not-a-repo
expect "a malformed --repo is a usage error" 2 'owner/name'

if [[ -s $writes ]]; then bad "the doctor never calls a write method" "$(cat "$writes")"; else ok "the doctor never calls a write method"; fi
if gh api -X POST "$api/labels" >/dev/null 2>&1 || gh api --method PATCH "$api" >/dev/null 2>&1 || gh api -f name=x "$api/labels" >/dev/null 2>&1; then
  bad "the fake gh refuses a write"
elif [[ $(wc -l <"$writes") == 3 ]]; then ok "the fake gh refuses and records a write"; else bad "the fake gh records each write" "$(cat "$writes")"; fi

echo "gate-doctor tests: $pass passed, $fail failed"
((fail == 0))
