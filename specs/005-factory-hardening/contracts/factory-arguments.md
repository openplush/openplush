# Contract: Factory arguments

`/factory` takes a feature, and later a grant (F-002). Before anything else a firing scans every argument for placeholders.

## Placeholder rule (US1)

A placeholder is either:

- an angle-bracketed word: `<window>`, `<feature>`;
- angle brackets containing a pipe: `<accepted | change it to ...>`.

The first match stops the firing before any branch, commit or comment. The stop names the matched text and the field it stands for. A blocker inside a task is not a placeholder: it is skipped and recorded as before.

## Grant (US6, waits on F-002 and ADR 0020)

Fields and their rules are in [data-model.md](../data-model.md#grant-us6-waits-on-f-002). The argument syntax is ADR 0020's to set.
