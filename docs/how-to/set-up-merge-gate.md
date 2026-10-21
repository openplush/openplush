# How to: set up the merge gate

This page takes a repository from no merge gate to one that comments on every pull request, and then to one that turns on auto-merge. The maintainer needs it once per repository, and again whenever the gate stops commenting.

## You will

Have every prerequisite of the [merge gate](../explanation/agents.md#the-merge-gate-tiers-and-who-merges) in place, confirmed by `scripts/gate-doctor.py`, with the gate rehearsing or live.

## Before you start

- Admin access to the repository: GitHub shows its merge settings, secrets and variables to admins only.
- Python 3.11 or newer, and the [GitHub CLI](https://cli.github.com) (`gh`) signed in with `gh auth login`. The doctor needs nothing else, so it runs the same in bash, zsh and PowerShell.
- A clone of the repository: the doctor reads the labels from `.github/labels.json` and the App's permissions from `.github/workflows/merge-gate.yml`.

## Steps

1. Run the doctor from the clone. It only reads; it never changes a setting.

   ```sh
   python3 scripts/gate-doctor.py --repo openplush/openplush
   ```

   On Windows, use `py` if `python3` is not on the path. Without `--repo`, it checks the repository `gh` sees in the current directory.

2. Read its output. It prints one line per prerequisite: each label in `.github/labels.json`, the repository's "Allow auto-merge" setting, the secrets `MERGE_GATE_APP_ID` and `MERGE_GATE_PRIVATE_KEY` that the gate's GitHub App is known by, and the repository variable `MERGE_GATE_MODE`.

   | State | Means |
   |---|---|
   | `ok` | In place |
   | `missing` | Not in place; the `fix:` line under it gives the exact command, and the settings page where there is one |
   | `unknown` | The doctor could not read it; the line names the access it needs. Never counted as in place |

3. Run each `fix:` command, or make the change on the settings page it names. For a missing App, create a GitHub App with the permissions the fix line lists, install it on the repository, and store its ID and private key as the two secrets. The doctor cannot see the installation itself, because GitHub shows an App's installations only to the App; if the App is not installed, the gate's next run fails at the step that creates its token.

4. Set `MERGE_GATE_MODE` to `dry-run` first. The gate then comments and labels `merge:ready` but never turns on auto-merge, and each of its comments says it is rehearsing. After reading a few of its verdicts, make it live:

   ```sh
   gh variable set MERGE_GATE_MODE --repo openplush/openplush --body on
   ```

   Setting it to `off`, or deleting it, stops the gate without a pull request.

5. Add the ruleset on `main` that [ADR 0015](../adr/0015-merge-gate.md) describes, requiring a pull request and the four CI checks. The doctor does not check it.

6. Run the doctor again.

## Check it worked

The doctor's last line reads `0 missing, 0 unknown`, and it exits with status 0. On the next pull request, the gate posts a comment that starts `Merge gate`.
