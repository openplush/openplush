# How to: start a feature's spec

<!-- index: copy the spec, plan and tasks templates into a new feature folder, for maintainers writing specs. -->

This page starts a new feature's folder under `specs/`, with its spec, plan and task list. Maintainers need it; a contributor picking up an existing task does not. [How agents work](../explanation/agents.md#from-an-idea-to-tasks) explains the flow itself.

## You will

Have a numbered feature folder with a spec, plan and tasks file copied from the templates, on a correctly named branch.

## Before you start

- A clone set up as in [Set up and run every check](../start/setup.md).
- The next free feature number: one more than the highest `NNN-` folder in [`specs/`](https://github.com/openplush/openplush/tree/main/specs).

## Steps

1. Branch from `main`, named as [the branch naming](pull-requests.md#3-name-the-branch-and-stack-it) describes:

   ```sh
   git switch -c 006/01-spec main
   ```

2. Make the feature folder and copy the spec template into it:

   ```sh
   mkdir specs/006-short-name
   cp specs/templates/spec-template.md specs/006-short-name/spec.md
   ```

3. Once the spec is clarified, copy the plan template the same way, to `plan.md`. Once the plan's decisions are recorded, copy the tasks template, to `tasks.md`.

4. Add the feature to the table in [`specs/README.md`](https://github.com/openplush/openplush/blob/main/specs/README.md): under Features, or under Repository process if no maker sees the change.

## Check it worked

`scripts/check.sh docs` passes, and the [status page](../start/status.md) lists the new feature once its `tasks.md` exists.
