# How to: set up Spec Kit to author specs

<!-- index: install `specify` and point it at a feature, for maintainers writing specs. -->

This page gets a clone ready to write or change a feature's spec, plan and tasks with [Spec Kit](https://github.com/github/spec-kit). Maintainers need it; a contributor picking up an existing task does not. [How agents work](../explanation/agents.md#from-an-idea-to-tasks-spec-kit) explains the Spec Kit flow itself.

## You will

Have `specify` installed and Spec Kit pointed at the feature you are working on, so its `/speckit-<command>` skills run in Claude Code.

## Before you start

- **Python 3.11 or newer**, and **[uv](https://docs.astral.sh/uv/)**.
- A clone set up as in [Set up and run every check](../start/setup.md).

## Steps

1. Install the Spec Kit command line and confirm it runs:

   ```sh
   uv tool install specify-cli
   specify version
   ```

   This repository uses Spec Kit's Claude Code integration with `sh` scripts, so its commands are the skills `/speckit-<command>`.

2. Point Spec Kit at your feature. It tracks the active feature in `.specify/feature.json`, which is local and ignored by git, so every clone sets it once:

   ```json
   {"feature_directory":"specs/001-pattern-pipeline"}
   ```

3. On Windows, Claude Code runs the `sh` scripts through Git Bash; `create-new-feature-branch.sh` and `check-prerequisites.sh` were verified to work there on 2026-10-13. If a script fails, switch to the PowerShell scripts:

   ```sh
   specify integration upgrade claude --script ps --force
   ```

4. When `/speckit-specify` starts a new feature, it creates a local `NNN-feature-slug` branch from `main`. Rename it to the pull request's branch name before pushing, as [the branch naming](pull-requests.md#3-name-the-branch-and-stack-it) describes:

   ```sh
   git branch -m 003/01-spec
   ```

## Check it worked

`specify version` prints a version, and Spec Kit's prerequisite script prints your feature's folder:

```sh
bash .specify/scripts/bash/check-prerequisites.sh --paths-only
```

Its `FEATURE_DIR` line ends in the folder you named in `.specify/feature.json`.
