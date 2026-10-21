#!/usr/bin/env bash
# Tick lag: tasks whose work has merged but whose box in tasks.md is still empty.
# A commit subject on the branch names its tasks in parentheses, as "(A001)" or
# "(A001, A002)"; every comma-separated item that is exactly a task ID counts, so
# a range "(A001–A003)" and a word such as "repair" are not read as tasks. Each
# such ID whose line in the branch's specs/*/tasks.md is unticked is printed with its feature,
# its age in days and the newest subject naming it, sorted by feature then ID.
# A report only, never a gate: exit 0 whenever it ran, 2 on a usage error.
# Usage: scripts/report-tick-lag.sh [--ref <ref>] [--now <epoch|date>] [root]
#   --ref  the branch to read (default main, else origin/main)
#   --now  the time ages are measured from (default the clock), for tests
#   root   the repository (default the current one)
# Tests: scripts/tests/report-tick-lag.test.sh
set -euo pipefail
export LC_ALL=C

usage_line="usage: scripts/report-tick-lag.sh [--ref <ref>] [--now <epoch|date>] [root]"

die_usage() {
  printf 'report-tick-lag: %s\n%s\n' "$1" "$usage_line" >&2
  exit 2
}

# to_epoch <epoch|date>: seconds since the epoch, UTC.
to_epoch() {
  if [[ $1 =~ ^[0-9]+$ ]]; then
    echo "$1"
  else
    date -u -d "$1" +%s 2>/dev/null || die_usage "--now: not an epoch or a date: $1"
  fi
}

# resolve_ref <ref or empty>: the ref to read; main, else origin/main, when empty.
resolve_ref() {
  local candidate
  if [[ -n $1 ]]; then
    git rev-parse --verify --quiet "$1^{commit}" >/dev/null || die_usage "--ref: no commit named $1"
    echo "$1"
    return
  fi
  for candidate in main origin/main; do
    if git rev-parse --verify --quiet "$candidate^{commit}" >/dev/null; then
      echo "$candidate"
      return
    fi
  done
  die_usage "neither main nor origin/main exists; pass --ref"
}

# open_tasks <ref>: "<feature> <ID>" for every unticked task line in the ref's
# specs/*/tasks.md, so the join never mixes the working tree with another branch.
open_tasks() {
  local f
  git ls-tree -r --name-only "$1" -- specs | grep -E '^specs/[^/]+/tasks\.md$' | sort |
    while IFS= read -r f; do
      git show "$1:$f" | sed -nE 's/^- \[ \] ([A-Z][0-9]{3}[a-z]?)([^0-9A-Za-z].*)?$/\1/p' |
        sed "s|^|$(basename "$(dirname "$f")") |"
    done
}

# named_ids <ref>: "<ID>\t<commit time>\t<subject>" for the newest subject naming each ID.
named_ids() {
  git log --format='%ct%x09%s' "$1" | awk -F '\t' '
    {
      ct = $1; subject = substr($0, length($1) + 2); rest = subject
      while (match(rest, /\([^()]*\)/)) {
        n = split(substr(rest, RSTART + 1, RLENGTH - 2), items, ",")
        for (i = 1; i <= n; i++) {
          id = items[i]; gsub(/^ +| +$/, "", id)
          if (id ~ /^[A-Z][0-9][0-9][0-9][a-z]?$/ && (!(id in when) || ct > when[id])) {
            when[id] = ct; line[id] = subject
          }
        }
        rest = substr(rest, RSTART + RLENGTH)
      }
    }
    END { for (id in when) printf "%s\t%s\t%s\n", id, when[id], line[id] }'
}

ref=""
now=""
root=""
while (($#)); do
  case $1 in
    --ref) (($# >= 2)) || die_usage "--ref needs a value"; ref=$2; shift 2 ;;
    --now) (($# >= 2)) || die_usage "--now needs a value"; now=$2; shift 2 ;;
    -h | --help) echo "$usage_line"; exit 0 ;;
    -*) die_usage "unknown option $1" ;;
    *) [[ -z $root ]] || die_usage "one root only"; root=$1; shift ;;
  esac
done

[[ -z $root ]] || cd "$root" || die_usage "no directory $root"
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || die_usage "not a git repository: $(pwd)"
cd "$(git rev-parse --show-toplevel)"
ref=$(resolve_ref "$ref")
now=$(to_epoch "${now:-$(date -u +%s)}")

lagging=$(awk -F '\t' -v now="$now" '
  NR == FNR { split($0, ft, " "); feature[ft[2]] = feature[ft[2]] " " ft[1]; next }
  $1 in feature {
    d = (now - $2) / 86400; age = int(d); if (d < 0 && d != age) age--
    n = split(substr(feature[$1], 2), fs, " ")
    for (i = 1; i <= n; i++) printf "%s %s %dd %s\n", fs[i], $1, age, $3
  }' <(open_tasks "$ref") <(named_ids "$ref") | sort -t ' ' -k1,1 -k2,2)

[[ -z $lagging ]] || echo "$lagging"
count=$(grep -c . <<<"$lagging" || true)
echo "tick-lag: $count unticked tasks named on $ref"
