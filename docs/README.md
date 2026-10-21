# OpenPlush documentation

This is the contributor book: for anyone changing OpenPlush, in Rust, TypeScript, test models or the docs. If you want to sew a plush rather than change the tool, this book is not for you: the maker guide is, from the first public release.

## Where to begin

New here? Read [Start here](start/README.md) first. It says what OpenPlush is and gives the path through this book, from [setting up](start/setup.md) to your first pull request. What works today is on the [status page](start/status.md), and nowhere else.

## How the book is organised

Pages are sorted by what you are doing when you open one, because a reader learning, reaching a goal, looking something up or trying to understand needs a different page each time ([Diátaxis](https://diataxis.fr)).

- [Explanation](explanation/README.md): why things are the way they are, such as [why seam lengths are pinned exactly](explanation/flattening.md).
- [How-to guides](how-to/README.md): one goal each, such as [scoping and opening a pull request](how-to/pull-requests.md).
- [Reference](reference/README.md): exact facts to look up, such as the [crates](reference/crates/README.md) and every term in the [glossary](reference/glossary.md).
- [Tutorials](tutorials/README.md): learning by doing, start to finish.
- [Architecture decision records](adr/README.md): each decision, the options weighed and why one won.

Specs and plans live in [`specs/`](https://github.com/openplush/openplush/tree/main/specs/), not here: they say what will be built and why. These pages describe what exists.
