# Close the Format Blind Spot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Break every `crates/**/*.rs` line over 100 characters into lines that fit, then add the CI step spec §1.2 decides: fail when any such line exceeds 100 characters, naming file, line and width.

**Architecture:** `rustfmt` silently emits a block verbatim when one of its lines cannot be brought within `max_width`, and `--check` reports no diff. The check targets that *cause*. Comparing `rustfmt --emit stdout` with the file cannot do it: a skipped block is emitted unchanged, so it compares equal exactly like a well-formatted one — the comparison only re-runs `cargo fmt --check`.

**Tech Stack:** Existing Rust/Cargo workspace, `rustfmt`, Bash, GitHub Actions.

**Spec:** [2026-10-04-structure-hygiene-design.md](../specs/2026-10-04-structure-hygiene-design.md) §1, with the remaining §2 extraction included below at Mohammed's request.

## Global Constraints

- Every line in `crates/**/*.rs` ends at column ≤ 100 after this plan. No exception list.
- `error_on_line_overflow = true` is **unstable** on stable and prints `Warning: can't set error_on_line_overflow = true` — verified. The gate does not depend on it.
- `cargo fmt --all --check` exits 0 at the end of this plan.
- `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings` is unchanged by this plan; every change must leave it green.
- `cargo test --workspace` is unchanged.
- No public API, route, schema, type, or test is changed.
- No new dependency.

## Review Focus

- **A new file containing a line `rustfmt` cannot break.** Today's `cargo fmt --check` exits 0; tomorrow's CI step exits 1. The diff is one bash step, not a toolchain change.
- **A macro invocation longer than 100 columns.** `json!({...})`, `sqlx::query("...")`, `tracing::warn!(...)`, and `#[utoipa::path(...)]` are the project's four long-line sources. They take one line each today; this plan puts each on multiple lines. The readable shape — which field on which line — is the executor's call, but every line must close.
- **Generated code longer than 100 columns.** The repo has none today (`specs/2026-10-04-structure-hygiene-design.md §1.3` records this as an OPEN trigger). If the executor hits a `// @generated` marker, stop and re-read §1.3.
- **A regression in the new check itself.** Ubuntu's default `awk` is `mawk`, which counts bytes; a line with `§` or `—` is wider in bytes than in characters, and rustfmt counts characters. The step drops UTF-8 continuation bytes and a CRLF checkout's `\r` before measuring, under `LC_ALL=C`, so both runners count what rustfmt counts.

---

## File Structure

| Path | Action | Responsibility after |
|---|---|---|
| `crates/**/*.rs` | modify | every line ≤ 100 cols |
| `.github/workflows/ci.yml` | modify | one new step after `Format` in `linux` and `windows` |

No file is created. No file is deleted. No file is moved.

---

## Task 1: Bring every `crates/**/*.rs` line to ≤ 100 columns

### Files

- Modify: every `crates/**/*.rs` containing a line over 100 columns
- Baseline `5c9692d`: 64 files, 259 lines: 128 in product, 131 in test apparatus (`tests/` and `fake-acp/`)

### Interfaces

- Consumes: the list of (file, line number, width) triples the spec measured.
- Produces: a tree where the Unicode character-count check below prints `0`.

---

- [x] **Step 1: Record the baseline**

Run from the repository root:

```bash
total=$(LC_ALL=C awk '{s=$0; sub(/\r$/, "", s); gsub(/[\200-\277]/, "", s)} length(s) > 100 {n++} END {print n+0}' $(find crates -name '*.rs'))
files=$(LC_ALL=C awk '{s=$0; sub(/\r$/, "", s); gsub(/[\200-\277]/, "", s)} length(s) > 100 {c[FILENAME]++} END {print length(c)+0}' $(find crates -name '*.rs'))
echo "baseline: $total overlong lines in $files files"
```

Expected at the baseline: `259 overlong lines in 64 files`. The exact numbers are the
acceptance threshold for Step 8.

- [x] **Step 2: List the affected files**

Run:

```bash
LC_ALL=C awk '{s=$0; sub(/\r$/, "", s); gsub(/[\200-\277]/, "", s)} length(s) > 100 {c[FILENAME]++} END {for (f in c) print c[f], f}' $(find crates -name '*.rs') | sort -rn
```

Expected at the baseline: 64 lines, the top one being
`crates/fake-acp/src/main.rs` with 40. `turns/turn.rs` has 18.

- [x] **Step 3: Pick the breaking strategy**

For each line, the executor chooses one of three breaks, in this order of
preference:

1. **Wrap a macro chain on its `.` boundaries.** `json!({...})` becomes
   ```rust
   json!({
       "k": "v",
   })
   ```
   `sqlx::query("UPDATE ...")?.bind(...)?.bind(...)?.execute(...)?` becomes
   ```rust
   sqlx::query("UPDATE ...")
       .bind(...)?
       .bind(...)?
       .execute(...).await?
   ```

2. **Split a function/method chain on its argument list** when (1) does not
   apply. Each argument on its own line, indented +4 from the receiver.

3. **Break an attribute argument list** (`#[utoipa::path(...)]`,
   `#[derive(...)]`) at a comma, one argument per line, indented.

Do **not** reorder fields. Do not rename. Do not delete.

- [x] **Step 4: Break `crates/shadows-core/src/turns/turn.rs` (18 lines)**

The file with the most overlong lines. After this step the file's
character-count check must report 0. The cargo gate must still pass.

- [x] **Step 5: Break `crates/shadows-core/src/design/store/vision.rs`**

This file is the one the spec measured the blind spot on. After this step,
`cargo fmt --all --check` must still exit 0 (the existing gate), and the
new step in Task 2 must also exit 0 when run locally against this file.

- [x] **Step 6: Break the remaining product files**

For each file with overlong lines under `crates/**/src/**` (the product
code), break every offending line. After each file the project's existing
gate must still pass:

```bash
cargo fmt --all --check
```

If a break breaks a build, fix the break — never silence the build.

- [x] **Step 7: Break test files**

For each file under `crates/*/tests/` and `crates/fake-acp/`, break every
overlong line. The same gate applies; tests run afterwards.

- [x] **Step 8: Confirm the baseline is zero**

Run:

```bash
LC_ALL=C awk '{s=$0; sub(/\r$/, "", s); gsub(/[\200-\277]/, "", s)} length(s) > 100 {n++} END {print n+0}' $(find crates -name '*.rs')
```

Expected: `0`. Any non-zero output names the file and line that the executor
missed. Fix and re-check.

- [x] **Step 9: Run the project's existing gate**

From the repository root:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo tree -e features,no-dev --workspace | grep test-support
```

All five must pass. The last prints nothing.

- [x] **Step 10: Commit**

```bash
git add crates/
git commit -m "style: bring every crates/**/*.rs line to ≤ 100 cols

cargo fmt --check passes today, but rustfmt silently emits a block
verbatim when one statement inside cannot be brought within max_width.
That block — and any hand-written dirt it contains — is outside the
check's reach.

Closing the blind spot starts by breaking the overlong lines.
The follow-up gate step rejects lines above 100 characters.

No code, route, schema, type or test changes.

"
```

---

## Task 2: Add the line-length step to CI

### Files

- Modify: `.github/workflows/ci.yml`

### Interfaces

- Consumes: every `crates/**/*.rs` file.
- Produces: a step after `Format`, in both `windows` and `linux`, that exits
  non-zero when any line exceeds 100 characters and prints a `::error` line
  naming file, line and width (spec §1.2).

---

- [x] **Step 1: Add the step after each `Format` step**

```yaml
      - name: Line length — what makes rustfmt skip a block
        shell: bash
        run: |
          find crates -name '*.rs' -not -path '*/target/*' -print0 \
            | LC_ALL=C xargs -0 awk '
                { s = $0; sub(/\r$/, "", s); gsub(/[\200-\277]/, "", s) }
                length(s) > 100 {
                  printf "::error file=%s,line=%d::%d characters; the limit is 100\n",
                    FILENAME, FNR, length(s)
                  bad = 1
                }
                END { exit bad }'
```

`LC_ALL=C` plus dropping UTF-8 continuation bytes counts characters on both
`mawk` (Ubuntu) and `gawk` (Git for Windows); `sub(/\r$/…)` keeps a CRLF
checkout from adding one.

- [x] **Step 2: Prove it in both directions**

Run the step's body locally on three trees:

| Tree | Expected | Seen |
|---|---|---|
| the tree after Task 1 | exit 0 | exit 0 |
| one line of exactly 100 characters, 20 of them `—` | exit 0 | exit 0 |
| the same line plus one character | `::error file=…,line=1::101 characters`, non-zero | exit 123, named the file |

- [x] **Step 3: Run the project's gate, then commit**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo tree -e features,no-dev --workspace | grep test-support
git diff --exit-code api/
```

```bash
git add .github/workflows/ci.yml
git commit -m "ci: fail on a crates line over 100 characters"
```

## Task 3: Complete the shared editor lifecycle (§2)

Modify `web/src/app/design/part-editor.tsx` and `outcome-editor.tsx`; add
`use-design-editor.ts` as the sole owner of their save/reload lifecycle.

- [x] Run the existing editor and reconnect integration tests before extraction: 12 passed.
- [x] Extract pinned create revisions, dirty input retention, command retry, conflict handling and explicit reload.
- [x] Keep entity content operations typed; keep Outcome part references as the optional relations configuration.
- [x] Preserve the authoritative-read-before-attempt-reset ordering.
- [x] Run the same integration tests unchanged after extraction: 12 passed.
- [x] Run the full Rust and Web gates; regenerate OpenAPI and client declarations together.
- [x] Update status and commit the remaining scoped work.

## Completion record — 2026-10-04

The inherited formatting commit was verified rather than repeated. Windows
gate: 385 Rust tests (one manual migration-copy test ignored), 180 Web tests,
both clippy modes, production build and feature isolation, formatting, Web
typecheck/lint/build, OpenAPI consistency and line-width RED/GREEN probes.
Remaining work is saved together as one scoped cleanup commit; independent
branch review remains pending per the standing user instruction.
