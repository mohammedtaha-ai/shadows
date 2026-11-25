# Codex commit recovery: the rejected-approval case (B-196)

Date: 2026-11-23 (Asia/Aden). Main at `4d69977d`. Codex `codex-cli 0.169.0`.

The 2026-11-22 run ([evidence](2026-11-22-codex-commit-recovery-live.md)) observed every
B-196 case except "the person rejects an approval for a denied Git write".

## Finding

That case cannot occur in the mode executors run in. A Codex conversation in `ask`
("Approve for me") sends `approvalsReviewer: "auto_review"` with every `turn/start`
(`crates/shadows-agent/src/codex/session.rs:118`). Codex's protocol describes
`auto_review` as a prompted subagent that approves or denies each request itself; requests
go to the person only with the reviewer `user`. The `read-only` mode uses `user` but sets
approval `never`, so nothing is asked there either. No approval card for a Codex Git write
reaches the person in either mode.

The reachable variant, Codex's own reviewer denying a scoped Git write, was not observed:
in the 2026-11-22 run it approved every scoped request.

## Outcome

B-196 is closed by the owner's decision on this finding. A reviewer denial stays
unobserved; the executor prompt's handling of a denied write (B-194) is covered by
`cargo test -p core-runs --lib the_first_prompt_says_how_an_attempt_ends`.
