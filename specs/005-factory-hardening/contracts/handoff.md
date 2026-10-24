# Contract: Handoff

The last output of every factory firing (US2), and of any session that stops at a stop condition.

## Sections, in order

1. **State**: merged PRs; open PRs with number, head SHA and base; the branch in hand.
2. **Next steps**: in order.
3. **Blockers**: each a decision prompt (facts; file and ID; one recommendation; the default if unanswered).
4. **Kickoff**: five lines or fewer, every field filled, the active grant included, so it passes the placeholder rule.

A firing that stops on its first step still emits all four; empty sections say "none".
