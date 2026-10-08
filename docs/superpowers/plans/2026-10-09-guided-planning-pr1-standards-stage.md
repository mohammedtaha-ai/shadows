# §23 PR 1 — Standards and the Stage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Shadows' base standards are compiled in. A project adds its own rules and parts from a Standards tab. The `design` service computes the project's stage. The Planner receives the standards at session open, receives them again when they change, and gets one stage line with every message. The conversation header shows the stage.

**Architecture:**
- **Standards:**
  - The base standards are `crates/shadows-core/src/design/standards.yaml`, parsed once with `yaml-rust2`.
  - Project additions are versioned rows, like Planner instructions (§13.8), owned by the `design` service.
- **Stage:** a pure function of the vision, the top-level parts and the effective standards.
- **Harness:** adds the standards to the session `append`. It compares the standards versions in `context_before_turn`, as it already compares instructions, and sends the stage line as its own block after every message.

**Tech Stack:** Rust (SQLx 0.9 on SQLite, axum + utoipa, yaml-rust2 0.13), React + TanStack Query in `web/`.

**Spec:** `docs/superpowers/specs/2026-10-08-guided-planning-design.md` §23.2, §23.4 (the delivery row for PR 1 is in §23.9).

## Global Constraints

- **Cargo commands:** prefix every one with `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/shadows-target CARGO_INCREMENTAL=0`.
- **Line length:** no Rust line over 100 columns.
- **Migrations:** never edit an existing one. This PR adds `0021_standards.sql` only.
- **Integration tests:** add no new test file under `tests/`; each is a ~49 MB binary. Extend the files this plan names.
- **API:** when `api/openapi.json` changes, run `npm run gen:api` in `web/` and commit both files.
- **Edit tools:** no prettier, and no Python or sed edits to source files.
- **Tests:** run targeted tests per task; the full gate (CLAUDE.md steps 1–6 plus `npm test`, `npm run lint`, `npm run build` in `web/`) runs once, in Task 6.
- **Contracts:** every service change updates its `contract.yaml` in the same commit, following `docs/codebase/contracts/TEMPLATE.yaml`.
- **Base standards, from §23.2:**
  - **Edits:** no route or tool edits them, and an addition never removes or weakens a base rule.
  - **Mandatory parts:**
    - `backend`: never waived.
    - `database`: never waived.
    - `api`: waivable "with the person's approval and a written reason".
    - `frontend`: waivable on the same terms.
- **Stage line:** about 50 tokens, written by Shadows, and sent with each of the person's messages.

## Rulings made while planning

Record each as a ledger `Ruling:` line at Task 1.

1. **YAML parser:** `yaml-rust2` becomes a product dependency of `shadows-core`.
   - Why: §23.2 names a YAML file, and the workspace already pins this maintained parser.
   - Also: the `Cargo.toml` comment "Never a product dependency" is rewritten.
   - Cost if wrong: swapping the parser, in one file.
2. **Stage scope:**
   - **Stages:** this PR's stage stops at `structure`. `roadmap` and `plans` arrive with PR 4 and PR 5, when a written structure exists to pass.
   - **Drift:** "unserved items" needs vision items (PR 2), and "map approved against an older vision" needs map approvals (PR 3). Both join `StageView` in those PRs.
   - **Waivers:** they arrive with the map gate (PR 3).
   - Cost if wrong: one added field and one added variant later.
3. **What counts as a mandatory part:** a mandatory part `p` is present when a top-level part has `kind` equal to `p`, after trimming. This is the field the person already edits on the map.
   - Cost if wrong: one comparison in `stage.rs`.
4. **Where additions are edited:** the Standards view is a workspace tab, beside Vision and Project map, as §23.9's "A Standards tab" says.
   - §23.2's "edited from the project's settings" is amended in place to "from the workspace's Standards tab".
5. **The stage line:** it is its own block, always last after focus and continue-plan. Tests that compare a turn's exact blocks gain it.
6. **The additions body** is `{ rules: [{ id, text, parts }], parts: [{ name, owns }] }`.
   - **Rule ids:** match `P<n>` (n ≥ 1) and are unique. `P` keeps them apart from the base `S<n>`.
   - **Part names:** match `^[a-z][a-z0-9-]*$`, are unique, and never repeat a base part.
   - **A rule's `parts`:** every name is a base or added part; an empty list means every part.
   - **Text:** trimmed text is non-empty.
   - **Refusals:** anything else is refused with `INVALID_COMMAND`, naming the item.

## Review Focus

1. **A project with no additions saved.** Expected: `GET …/standards` answers the base standards with `additions: null`, and the stage still names the four base parts. Test: `standards_without_additions_are_the_base` (Task 2).
2. **Additions saved while a session is open but before its first turn.** Expected: the first turn carries the new standards once. Test: `changed_standards_reach_the_next_turn_once` (Task 4).
3. **A thread whose invocations predate this migration** (`standards_version` NULL). Expected: it is sent the standards once, not on every turn. Test: `a_thread_from_before_the_standards_gets_them_once` (Task 4).
4. **A part whose `kind` has whitespace or different case.** Expected: `" backend "` counts and `"Backend"` does not; kinds are names, not prose. Test: `stage_counts_a_trimmed_kind_and_no_other_case` (Task 3).
5. **An addition naming a base part, or a rule naming an unknown part.** Expected: refused with `INVALID_COMMAND`, nothing saved. Test: `additions_cannot_repeat_a_base_part_or_name_an_unknown_one` (Task 2).

---

### Task 1: Base standards compiled in

**Files:**
- Create: `crates/shadows-core/src/design/standards.yaml`
- Create: `crates/shadows-core/src/design/standards.rs`
- Modify: `crates/shadows-core/src/design/mod.rs` (add `mod standards;`, re-exports)
- Modify: `crates/shadows-core/src/lib.rs` (re-export the new public types next to the other design types)
- Modify: `crates/shadows-core/Cargo.toml` (`yaml-rust2 = { workspace = true }` under `[dependencies]`; keep it in `[dev-dependencies]` too only if `tests/contracts.rs` needs it there, which it does not once it is a normal dependency, so remove it from dev)
- Modify: `Cargo.toml` (root): the comment above `yaml-rust2`
- Modify: `docs/codebase/README.md` (row for `standards.rs`)

**Interfaces:**
- Produces:
  ```rust
  // design/standards.rs, all pub and re-exported from shadows_core
  #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
  pub struct MandatoryPart { pub name: String, pub owns: String, pub waivable: bool }
  #[derive(.. same derives ..)]
  pub struct StandardRule { pub id: String, pub text: String, pub parts: Vec<String> }
  #[derive(.. same derives ..)]
  pub struct ContractTemplate { pub rules: Vec<String>, pub shape: Vec<String> }
  #[derive(.. same derives ..)]
  pub struct BaseStandards {
      pub version: i64,
      pub parts: Vec<MandatoryPart>,
      pub rules: Vec<StandardRule>,
      pub contract_template: ContractTemplate,
  }
  pub fn base() -> &'static BaseStandards
  ```

- [ ] **Step 1: Write the failing unit test** at the bottom of `standards.rs` (the file holds only the test and `mod`-level `pub fn base()` stub returning `todo!()` so it compiles):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_standards_parse_with_the_four_mandatory_parts() {
        let b = base();
        assert!(b.version >= 1);
        let parts: Vec<(&str, bool)> =
            b.parts.iter().map(|p| (p.name.as_str(), p.waivable)).collect();
        assert_eq!(
            parts,
            [("backend", false), ("database", false), ("api", true), ("frontend", true)]
        );
        let ids: Vec<&str> = b.rules.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.len() >= 5 && ids.iter().all(|id| id.starts_with('S')), "{ids:?}");
        for rule in &b.rules {
            for part in &rule.parts {
                assert!(b.parts.iter().any(|p| &p.name == part), "{} names {part}", rule.id);
            }
        }
        assert!(b.contract_template.shape.contains(&"obligations".to_string()));
        assert!(!b.contract_template.rules.is_empty());
    }
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/shadows-target CARGO_INCREMENTAL=0 cargo test -p shadows-core --lib standards`
Expected: FAIL, panicked at `not yet implemented`.

- [ ] **Step 3: Write `standards.yaml`**

```yaml
# Shadows' base standards (spec §23.2). Compiled into the binary; changed only
# by a reviewed PR, and `version` goes up by one with every change. No route or
# tool edits this file. A project's additions never remove or weaken it.
version: 1

parts:
  - name: backend
    owns: "All logic: one core; every operation is a method of one service."
    waivable: false
  - name: database
    owns: >-
      The schema and storage: the only part that touches the database;
      migrations are append-only.
    waivable: false
  - name: api
    owns: >-
      The only path between the frontend and the backend, with a versioned
      contract.
    waivable: true
  - name: frontend
    owns: "The interface only, with no business rules."
    waivable: true

rules:
  - id: S1
    text: >-
      Each service owns its tables and reads no other service's; it asks the
      owning service.
    parts: [backend, database]
  - id: S2
    text: "The frontend reaches data only through the api."
    parts: [frontend, api]
  - id: S3
    text: "Every part has a contract, written to the contract template below."
    parts: []
  - id: S4
    text: "Migrations are never edited; a change is a new migration."
    parts: [database]
  - id: S5
    text: >-
      Ordering is explicit: a sequence, an ordinal or another stable key,
      never insertion order.
    parts: [backend, database]
  - id: S6
    text: "The backend and the database are separate parts, never one file."
    parts: [backend, database]

contract_template:
  rules:
    - >-
      Trace, never skim: every claim comes from code that was read; when a
      comment disagrees with the code, describe the code and record a gap.
    - >-
      A gap names a symbol in this source (function, type, constant), never a
      line number; a suspicion about another source is an open question.
    - >-
      An obligation names the test that holds it, or says none or unknown;
      read test bodies, not only their names.
    - >-
      Reference a type owned elsewhere by its source; do not copy its
      members. Describe only what this source owns.
    - >-
      The header says what only this source can do: its responsibility,
      invariants, boundaries and traps, not a list of functions.
    - >-
      Group functions as the source reads best (reads, writes, checks…);
      mark a function private when its privacy holds an invariant.
    - >-
      Two paths that must give the same answer are an agreement, with its
      test; one that does not hold is also a gap.
    - "Keep the file valid YAML; quote signatures that contain ': '."
    - "Contradict yourself nowhere: every section agrees with the others."
  shape:
    - header
    - shapes
    - functions
    - enums
    - obligations
    - agreements
    - not_the_caller's
    - gaps
    - open_questions
    - tests
```

- [ ] **Step 4: Write `standards.rs`**

```rust
//! One job: Shadows' base standards (spec §23.2), the compiled-in
//! `standards.yaml` read once into typed values.
//!
//! Trap: the file is code, not data. Its `version` is what a turn records;
//! changing a rule without raising it means no running session hears of the
//! change.

use std::sync::OnceLock;

use yaml_rust2::{Yaml, YamlLoader};

const SOURCE: &str = include_str!("standards.yaml");

// (the four structs from Interfaces, each with a one-line doc comment)

/// The base standards, parsed on first use. The file is compiled in and
/// tested, so a parse failure is a build defect: it panics.
pub fn base() -> &'static BaseStandards {
    static BASE: OnceLock<BaseStandards> = OnceLock::new();
    BASE.get_or_init(|| parse(SOURCE).expect("standards.yaml is valid"))
}

fn parse(source: &str) -> Result<BaseStandards, String> {
    let docs = YamlLoader::load_from_str(source).map_err(|e| e.to_string())?;
    let doc = docs.first().ok_or("empty standards.yaml")?;
    Ok(BaseStandards {
        version: doc["version"].as_i64().ok_or("version")?,
        parts: list(&doc["parts"])?
            .iter()
            .map(|p| {
                Ok(MandatoryPart {
                    name: text(&p["name"])?,
                    owns: text(&p["owns"])?,
                    waivable: p["waivable"].as_bool().ok_or("waivable")?,
                })
            })
            .collect::<Result<_, String>>()?,
        rules: list(&doc["rules"])?
            .iter()
            .map(|r| {
                Ok(StandardRule {
                    id: text(&r["id"])?,
                    text: text(&r["text"])?,
                    parts: strings(&r["parts"])?,
                })
            })
            .collect::<Result<_, String>>()?,
        contract_template: ContractTemplate {
            rules: strings(&doc["contract_template"]["rules"])?,
            shape: strings(&doc["contract_template"]["shape"])?,
        },
    })
}

fn list(y: &Yaml) -> Result<&Vec<Yaml>, String> {
    y.as_vec().ok_or_else(|| format!("expected a list, found {y:?}"))
}

fn text(y: &Yaml) -> Result<String, String> {
    y.as_str().map(str::to_owned).ok_or_else(|| format!("expected text, found {y:?}"))
}

fn strings(y: &Yaml) -> Result<Vec<String>, String> {
    list(y)?.iter().map(text).collect()
}
```

Add `mod standards;` and `pub use standards::{BaseStandards, ContractTemplate, MandatoryPart, StandardRule, base as base_standards};` to `design/mod.rs`. Re-export these from `lib.rs` wherever the other design types are re-exported.

Cargo:
- In `crates/shadows-core/Cargo.toml`, add `yaml-rust2 = { workspace = true }` under `[dependencies]` and remove it from `[dev-dependencies]`.
- In the root `Cargo.toml`, replace the two comment lines above `yaml-rust2` with:
  ```toml
  # Parses Shadows' compiled-in base standards (spec §23.2) and, in tests, each
  # service's `contract.yaml` (spec §14.7): the maintained successor of `serde_yaml`.
  ```

Code map: add a row after `design/ops.rs`:
`| \`crates/shadows-core/src/design/standards.rs\` | the compiled-in base standards | \`crates/shadows-core/src/design/standards.rs\` |`

- [ ] **Step 5: Run it to see it pass**

Run: same command as Step 2, then `… cargo test -p shadows --test codemap`
Expected: `1 passed` and the codemap test passes.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/shadows-core docs/codebase/README.md
git commit -m "§23.2: Shadows' base standards compiled in"
```

---

### Task 2: Project additions — table, service, routes

**Files:**
- Create: `crates/shadows-core/migrations/0021_standards.sql`
- Create: `crates/shadows-core/src/design/store/standards.rs`
- Modify: `crates/shadows-core/src/design/store/mod.rs` (`mod standards;`)
- Modify: `crates/shadows-core/src/design/standards.rs` (additions types, `validate`, `EffectiveStandards`)
- Modify: `crates/shadows-core/src/design/mod.rs` (two service methods, re-exports)
- Modify: `crates/shadows-http/src/design.rs`, `crates/shadows-http/src/lib.rs` (two routes)
- Modify: `crates/shadows-core/src/design/contract.yaml`
- Modify: `api/openapi.json`, `web/src/api/schema.d.ts` (generated)
- Test: `crates/shadows-core/tests/design_vision.rs`, `crates/shadows/tests/design_routes.rs`

**Interfaces:**
- Consumes: `base()`, `StandardRule` (Task 1).
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
  pub struct AdditionalPart { pub name: String, pub owns: String }
  #[derive(.., Default, ..)]
  pub struct StandardsAdditions { pub rules: Vec<StandardRule>, pub parts: Vec<AdditionalPart> }
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  pub struct StandardsAdditionsVersion {
      #[serde(skip)] pub id: String,
      pub number: i64,
      pub content: StandardsAdditions,
      pub created_at: String,
  }
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  pub struct EffectiveStandards {
      pub base: BaseStandards,
      pub additions: Option<StandardsAdditionsVersion>,
  }
  // design/standards.rs
  pub(crate) fn validate(additions: &StandardsAdditions) -> Result<(), String>
  impl EffectiveStandards {
      /// Base parts, then added parts, by name.
      pub fn mandatory_parts(&self) -> Vec<&str>
  }
  // Storage (design/store/standards.rs)
  pub async fn save_standards_additions(&self, ctx: &CommandContext, project: &ProjectId,
      content: &StandardsAdditions) -> Result<StandardsAdditionsVersion, StorageError>
  pub async fn current_standards_additions(&self, project: &ProjectId)
      -> Result<Option<StandardsAdditionsVersion>, StorageError>
  // Design service
  pub async fn standards(&self, project: &ProjectId) -> Result<EffectiveStandards, CoreError>
  pub async fn save_standards_additions(&self, command_id: String, project: &ProjectId,
      content: StandardsAdditions) -> Result<StandardsAdditionsVersion, CoreError>
  ```
  HTTP: `GET /api/projects/{id}/standards` → `EffectiveStandards`; `PUT /api/projects/{id}/standards/additions` with `{ command_id, content }` → `StandardsAdditionsVersion`. Event `ProjectStandardsSaved` with payload `{ "number": n }`.

- [ ] **Step 1: Write the failing core tests** in `crates/shadows-core/tests/design_vision.rs`. Add a helper `core_and_project(tmp) -> (Arc<AppCore>, ProjectId)` built exactly like the first test's setup, and use it:

```rust
use shadows_core::{AdditionalPart, StandardRule, StandardsAdditions};

fn rule(id: &str, parts: &[&str]) -> StandardRule {
    StandardRule {
        id: id.into(),
        text: "Every payment is logged.".into(),
        parts: parts.iter().map(|p| p.to_string()).collect(),
    }
}

#[tokio::test]
async fn standards_without_additions_are_the_base() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, project) = core_and_project(&tmp).await;
    let standards = core.design().standards(&project).await.unwrap();
    assert_eq!(&standards.base, shadows_core::base_standards());
    assert_eq!(standards.additions, None);
    assert_eq!(standards.mandatory_parts(), ["backend", "database", "api", "frontend"]);
}

#[tokio::test]
async fn each_additions_save_is_a_new_version_and_a_replay_saves_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, project) = core_and_project(&tmp).await;
    let content = StandardsAdditions {
        rules: vec![rule("P1", &["billing"])],
        parts: vec![AdditionalPart { name: "billing".into(), owns: "Payments.".into() }],
    };
    let design = core.design();
    let first = design.save_standards_additions("a1".into(), &project, content.clone());
    let first = first.await.unwrap();
    assert_eq!(first.number, 1);
    let replay = design.save_standards_additions("a1".into(), &project, content.clone());
    assert_eq!(replay.await.unwrap(), first);
    let second = design.save_standards_additions("a2".into(), &project, Default::default());
    assert_eq!(second.await.unwrap().number, 2);
    let standards = design.standards(&project).await.unwrap();
    assert_eq!(standards.additions.unwrap().number, 2);
    let mut changed = content;
    changed.parts[0].owns = "Other.".into();
    let conflict = design.save_standards_additions("a1".into(), &project, changed).await;
    assert!(matches!(conflict, Err(CoreError::CommandConflict)), "{conflict:?}");
}

#[tokio::test]
async fn additions_cannot_repeat_a_base_part_or_name_an_unknown_one() {
    let tmp = tempfile::tempdir().unwrap();
    let (core, project) = core_and_project(&tmp).await;
    let refused = [
        StandardsAdditions {
            parts: vec![AdditionalPart { name: "backend".into(), owns: "x".into() }],
            ..Default::default()
        },
        StandardsAdditions { rules: vec![rule("P1", &["billing"])], ..Default::default() },
        StandardsAdditions { rules: vec![rule("S1", &[])], ..Default::default() },
        StandardsAdditions { rules: vec![rule("P1", &[]), rule("P1", &[])], ..Default::default() },
        StandardsAdditions {
            parts: vec![AdditionalPart { name: "Billing".into(), owns: "x".into() }],
            ..Default::default()
        },
        StandardsAdditions {
            rules: vec![StandardRule { text: "  ".into(), ..rule("P1", &[]) }],
            ..Default::default()
        },
    ];
    for (i, content) in refused.into_iter().enumerate() {
        let id = format!("bad-{i}");
        let error = core.design().save_standards_additions(id, &project, content).await;
        assert!(
            matches!(&error, Err(CoreError::Refused { code: shadows_core::ErrorCode::InvalidCommand, .. })),
            "case {i}: {error:?}"
        );
    }
    assert_eq!(core.design().standards(&project).await.unwrap().additions, None);
}
```

(If `CoreError::CommandConflict` is spelled differently, use the variant `StorageError::CommandConflict` converts to. Find it with `where_is CoreError`.)

- [ ] **Step 2: Run them to see them fail**

Run: `… cargo test -p shadows-core --test design_vision`
Expected: compile error, `cannot find type StandardsAdditions`.

- [ ] **Step 3: Write the migration** `0021_standards.sql`

```sql
-- §23.2: a project's additions to Shadows' base standards, versioned like
-- Planner instructions; and, per invocation, the standards a turn ran under.
CREATE TABLE standards_additions_version (
    id           TEXT PRIMARY KEY,
    project_id   TEXT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    number       INTEGER NOT NULL CHECK (number >= 1),
    content_json TEXT NOT NULL CHECK (json_valid(content_json)),
    created_at   TEXT NOT NULL,
    UNIQUE (project_id, number)
);
ALTER TABLE agent_invocation ADD COLUMN standards_version INTEGER NULL;
ALTER TABLE agent_invocation ADD COLUMN standards_additions_version_id TEXT NULL
    REFERENCES standards_additions_version(id);
```

- [ ] **Step 4: Write the types and `validate`** in `standards.rs` (Interfaces above):

```rust
/// Ruling 6 of the PR 1 plan: every refusal names the item it refuses.
pub(crate) fn validate(additions: &StandardsAdditions) -> Result<(), String> {
    let base = base();
    let mut names: Vec<&str> = base.parts.iter().map(|p| p.name.as_str()).collect();
    for part in &additions.parts {
        let name = part.name.as_str();
        let shaped = name.starts_with(|c: char| c.is_ascii_lowercase())
            && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !shaped {
            return Err(format!("part \"{name}\": a name is lowercase letters, digits and -"));
        }
        if names.contains(&name) {
            return Err(format!("part \"{name}\" is already a mandatory part"));
        }
        if part.owns.trim().is_empty() {
            return Err(format!("part \"{name}\" needs what it owns"));
        }
        names.push(name);
    }
    let mut ids: Vec<&str> = Vec::new();
    for rule in &additions.rules {
        let id = rule.id.as_str();
        let numbered = id.strip_prefix('P').and_then(|n| n.parse::<u32>().ok())
            .is_some_and(|n| n >= 1 && id == format!("P{n}"));
        if !numbered {
            return Err(format!("rule \"{id}\": a project rule's id is P1, P2, …"));
        }
        if ids.contains(&id) {
            return Err(format!("rule {id} appears twice"));
        }
        if rule.text.trim().is_empty() {
            return Err(format!("rule {id} has no text"));
        }
        if let Some(unknown) = rule.parts.iter().find(|p| !names.contains(&p.as_str())) {
            return Err(format!("rule {id} names \"{unknown}\", which is not a part"));
        }
        ids.push(id);
    }
    Ok(())
}

impl EffectiveStandards {
    pub fn mandatory_parts(&self) -> Vec<&str> {
        let added = self.additions.iter().flat_map(|a| &a.content.parts);
        (self.base.parts.iter().map(|p| p.name.as_str()))
            .chain(added.map(|p| p.name.as_str()))
            .collect()
    }
}
```

- [ ] **Step 5: Write the store** `design/store/standards.rs`, the same shape as `instructions/store.rs`. It holds `save_standards_additions` and `current_standards_additions` (both joining `project … removed_at IS NULL` like that file), plus a private `load_version`:
  - **Classify:** `classify(conn, &ctx, "Project", project)`.
  - **Project:** it must exist, or `NotFound("project")`.
  - **Number:** `COALESCE(MAX(number),0)+1`.
  - **Row:** insert it with `serde_json::to_string(content)`.
  - **Event:** `DurableEvent::new("ProjectStandardsSaved", Actor::user(&ctx.principal_id)).with_project(&project).with_payload(json!({ "number": number }))`.
  - **Command record:** `record_command(conn, &ctx, "Project", project, "StandardsAdditions", &id, &ts)`.
  - **Return:** `load_version`.
  - **Header:** `//! One job: a project's standards additions (spec §23.2), versioned rows.`

- [ ] **Step 6: Write the service methods** in `design/mod.rs`:

```rust
    /// The project's effective standards (§23.2): the base, and its latest
    /// additions if it saved any.
    pub async fn standards(&self, project: &ProjectId) -> Result<EffectiveStandards, CoreError> {
        Ok(EffectiveStandards {
            base: standards::base().clone(),
            additions: self.storage.current_standards_additions(project).await?,
        })
    }

    /// Saves the project's next additions version: "StandardsAdditionsSave",
    /// params { project, content }. Validation refuses before any write.
    pub async fn save_standards_additions(
        &self,
        command_id: String,
        project: &ProjectId,
        content: StandardsAdditions,
    ) -> Result<StandardsAdditionsVersion, CoreError> {
        standards::validate(&content).map_err(|message| CoreError::Refused {
            code: crate::ErrorCode::InvalidCommand,
            message,
        })?;
        let params = serde_json::json!({ "project": project, "content": content });
        let ctx = user_command(command_id, "StandardsAdditionsSave", params);
        Ok(self.storage.save_standards_additions(&ctx, project, &content).await?)
    }
```

- [ ] **Step 7: Run the core tests**

Run: `… cargo test -p shadows-core --test design_vision`
Expected: all pass (the three new tests and the existing ones).

- [ ] **Step 8: Write the failing route test** in `crates/shadows/tests/design_routes.rs`:

```rust
#[tokio::test]
async fn standards_routes_read_the_base_and_save_additions() {
    let app = app::test_app().await;
    let standards = format!("/api/projects/{}/standards", app.project);
    let (status, body) = app::call(&app, "GET", &standards, None).await;
    assert_eq!(status, 200);
    assert_eq!(body["base"]["parts"][0]["name"], "backend");
    assert_eq!(body["additions"], Value::Null);
    let save = format!("{standards}/additions");
    let content = json!({ "rules": [], "parts": [{ "name": "billing", "owns": "Payments." }] });
    let request = json!({ "command_id": "s1", "content": content });
    let (status, saved) = app::call(&app, "PUT", &save, Some(request)).await;
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["number"], 1);
    assert!(saved.get("id").is_none());
    let bad = json!({ "command_id": "s2", "content": { "rules": [], "parts": [
        { "name": "api", "owns": "x" } ] } });
    let (status, failure) = app::call(&app, "PUT", &save, Some(bad)).await;
    assert_eq!((status, failure["code"].as_str()), (400, Some("INVALID_COMMAND")));
    let events = app.storage.read_project_events_after(EventCursor(0), &app.project, 100);
    let kinds: Vec<String> = events.await.unwrap().into_iter().map(|e| e.kind).collect();
    assert_eq!(kinds, ["ProjectStandardsSaved"]);
}
```

(If `INVALID_COMMAND` maps to another status in `failure/`, assert that status. Read it with `where_is ErrorCode`, not by guessing.)

- [ ] **Step 9: Write the routes** in `crates/shadows-http/src/design.rs`, modelled on `instructions.rs`:
  - `get_standards` → `s.core.design().standards(&project)`.
  - `save_standards_additions` takes `{ command_id: String, content: StandardsAdditions }` and calls `s.core.design().save_standards_additions(…)`.
  - utoipa paths: `/api/projects/{id}/standards` (get) and `/api/projects/{id}/standards/additions` (put), tag `projects`.
  - Responses: 200; 400 `INVALID_COMMAND`; 404 for an unknown project; 409 `COMMAND_CONFLICT`; 500.
  - Register both with `.routes(routes!(design::get_standards))` and `.routes(routes!(design::save_standards_additions))` in `lib.rs`, next to the other `design::` routes.

- [ ] **Step 10: Run the route test, then regenerate the API**

Run:
1. `… cargo test -p shadows --test design_routes`
2. `… cargo test -p shadows --test openapi` (it rewrites or checks `api/openapi.json`; follow its message)
3. `cd web && npm run gen:api`

Expected: everything passes, and `git status` shows `api/openapi.json` and `web/src/api/schema.d.ts` modified.

- [ ] **Step 11: Update `design/contract.yaml`**
  - **shapes:** `StandardsAdditions`, `AdditionalPart`, `StandardsAdditionsVersion` (trap: `id` is `#[serde(skip)]`), `EffectiveStandards`.
  - **service functions:** `standards` and `save_standards_additions`, with the refusal list.
  - **store functions:** `save_standards_additions` and `current_standards_additions`. The latter carries a comment that Harness reads it too.
  - **called_by_other_services:** `current_standards_additions`, read by Harness `Setups` and by Turns `record`.
  - **obligations:**
    - "an addition never repeats a base part or names an unknown one; refused before any write", tested by `additions_cannot_repeat_a_base_part_or_name_an_unknown_one`;
    - "each save is the next number; a replay saves nothing; the event carries the number only", tested by `each_additions_save_is_a_new_version_and_a_replay_saves_nothing` and `standards_routes_read_the_base_and_save_additions`.
  - **tests:** the four new tests, each with what its body checks.

Then run `… cargo test -p shadows-core --test contracts`. Expected: pass.

- [ ] **Step 12: Commit**

```bash
git add crates api web/src/api/schema.d.ts
git commit -m "§23.2: a project's standards additions, versioned, with their routes"
```

---

### Task 3: The stage

**Files:**
- Create: `crates/shadows-core/src/design/stage.rs`
- Create: `crates/shadows-core/src/design/store/stage.rs`
- Modify: `crates/shadows-core/src/design/mod.rs`, `design/store/mod.rs`, `crates/shadows-core/src/lib.rs`
- Modify: `crates/shadows-http/src/design.rs`, `lib.rs` (route `GET /api/projects/{id}/stage`)
- Modify: `crates/shadows-core/src/design/contract.yaml`, `docs/codebase/README.md`
- Modify: generated `api/openapi.json`, `web/src/api/schema.d.ts`
- Test: `crates/shadows-core/tests/design_parts.rs`, `crates/shadows/tests/design_routes.rs`

**Interfaces:**
- Consumes: `EffectiveStandards::mandatory_parts` (Task 2), `VisionContent`.
- Produces:
  ```rust
  // design/stage.rs
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "snake_case")]
  pub enum Stage { Idea, Vision, Map, Structure }
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  pub struct StageView {
      pub stage: Stage,
      /// Vision field names (`users`, `technical_direction`) at `vision`;
      /// mandatory part names at `map`; empty otherwise.
      pub missing: Vec<String>,
  }
  pub(crate) fn compute(vision: &VisionContent, kinds: &[String], required: &[&str]) -> StageView
  /// The line a turn carries (§23.4 3).
  pub(crate) fn line(view: &StageView) -> String
  // Storage (design/store/stage.rs)
  pub async fn project_stage(&self, project: &ProjectId) -> Result<StageView, StorageError>
  // Design service
  pub async fn stage(&self, project: &ProjectId) -> Result<StageView, CoreError>
  ```

- [ ] **Step 1: Write the failing tests** in `crates/shadows-core/tests/design_parts.rs`:

```rust
use shadows_core::{Stage, StageView, VisionContent};

fn kinded(kind: &str) -> DesignOp {
    DesignOp::PartCreate {
        id: PartId::generate(),
        parent: None,
        before: None,
        content: PartContent { kind: Some(kind.into()), ..content(kind) },
    }
}

fn vision(users: &str) -> DesignOp {
    DesignOp::VisionPut {
        content: VisionContent {
            purpose: "Sell books".into(),
            users: users.into(),
            goals: "G".into(),
            boundaries: "B".into(),
            technical_direction: "Rust".into(),
        },
    }
}

fn view(stage: Stage, missing: &[&str]) -> StageView {
    StageView { stage, missing: missing.iter().map(|m| m.to_string()).collect() }
}

#[tokio::test]
async fn the_stage_walks_idea_vision_map_structure() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("s.sqlite3")).await;
    let p = project(&core, "stage", tmp.path()).await;
    let stage = || core.design().stage(&p);
    assert_eq!(stage().await.unwrap(), view(Stage::Idea, &[]));
    edit(&core, &p, 0, vec![vision("  ")]).await;
    assert_eq!(stage().await.unwrap(), view(Stage::Vision, &["users"]));
    edit(&core, &p, 1, vec![vision("Readers"), kinded("backend")]).await;
    assert_eq!(stage().await.unwrap(), view(Stage::Map, &["database", "api", "frontend"]));
    edit(&core, &p, 2, vec![kinded("database"), kinded("api"), kinded("frontend")]).await;
    assert_eq!(stage().await.unwrap(), view(Stage::Structure, &[]));
}

#[tokio::test]
async fn stage_counts_a_trimmed_kind_and_no_other_case() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("s.sqlite3")).await;
    let p = project(&core, "kinds", tmp.path()).await;
    let ops = vec![vision("Readers"), kinded(" backend "), kinded("Database")];
    edit(&core, &p, 0, ops).await;
    let missing = core.design().stage(&p).await.unwrap().missing;
    assert_eq!(missing, ["database", "api", "frontend"]);
}

#[tokio::test]
async fn an_added_part_is_mandatory_for_the_stage() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("s.sqlite3")).await;
    let p = project(&core, "added", tmp.path()).await;
    let parts = ["backend", "database", "api", "frontend"].map(kinded);
    edit(&core, &p, 0, [vec![vision("Readers")], parts.to_vec()].concat()).await;
    let billing = shadows_core::AdditionalPart { name: "billing".into(), owns: "x".into() };
    let content = shadows_core::StandardsAdditions { parts: vec![billing], rules: vec![] };
    core.design().save_standards_additions("b".into(), &p, content).await.unwrap();
    assert_eq!(core.design().stage(&p).await.unwrap(), view(Stage::Map, &["billing"]));
}
```

(`edit(core, p, rev, ops)` is the file's existing helper. If its revision argument works differently, follow the helper. `kinded` reuses the file's `content(title)` for the other fields.)

Add to the route test in `design_routes.rs`, at the end of `standards_routes_read_the_base_and_save_additions`:
```rust
    let stage = format!("/api/projects/{}/stage", app.project);
    assert_eq!(
        app::call(&app, "GET", &stage, None).await,
        (200, json!({ "stage": "idea", "missing": [] }))
    );
```

- [ ] **Step 2: Run them to see them fail**

Run: `… cargo test -p shadows-core --test design_parts stage`
Expected: compile error, `cannot find type Stage`.

- [ ] **Step 3: Write `stage.rs`**

```rust
//! One job: a project's stage (spec §23.4), computed from what the database
//! holds, never from the conversation.
//!
//! This PR's stages end at `structure`; `roadmap` and `plans` arrive with the
//! structure write (§23.9 PR 4–5), drift with vision items and map approval.

use super::VisionContent;

// (Stage and StageView from Interfaces, with their derives and docs)

pub(crate) fn compute(vision: &VisionContent, kinds: &[String], required: &[&str]) -> StageView {
    let fields = [
        ("purpose", &vision.purpose),
        ("users", &vision.users),
        ("goals", &vision.goals),
        ("boundaries", &vision.boundaries),
        ("technical_direction", &vision.technical_direction),
    ];
    let empty: Vec<String> = fields
        .iter()
        .filter(|(_, text)| text.trim().is_empty())
        .map(|(name, _)| name.to_string())
        .collect();
    if empty.len() == fields.len() {
        return StageView { stage: Stage::Idea, missing: Vec::new() };
    }
    if !empty.is_empty() {
        return StageView { stage: Stage::Vision, missing: empty };
    }
    let missing: Vec<String> = required
        .iter()
        .filter(|name| !kinds.iter().any(|k| k.trim() == **name))
        .map(|name| name.to_string())
        .collect();
    if !missing.is_empty() {
        return StageView { stage: Stage::Map, missing };
    }
    StageView { stage: Stage::Structure, missing: Vec::new() }
}

pub(crate) fn line(view: &StageView) -> String {
    let missing = view.missing.join(", ");
    match view.stage {
        Stage::Idea => "[Shadows] Stage: idea. Nothing is written yet: discuss the idea; \
                        when the person asks how to start, walk the vision fields one at a time."
            .into(),
        Stage::Vision => format!("[Shadows] Stage: vision. Missing vision fields: {missing}."),
        Stage::Map => format!("[Shadows] Stage: map. Missing mandatory parts: {missing}."),
        Stage::Structure => {
            "[Shadows] Stage: structure. The vision and map are complete; \
             the structure is not written yet."
                .into()
        }
    }
}
```

- [ ] **Step 4: Write the store read** `design/store/stage.rs`:

```rust
//! One job: reading what a project's stage is computed from (spec §23.4).

use crate::db::{Storage, StorageError};
use crate::design::{StageView, stage, standards};
use crate::projects::ProjectId;

impl Storage {
    /// The project's stage. Also read by Harness before each turn.
    pub async fn project_stage(&self, project: &ProjectId) -> Result<StageView, StorageError> {
        let vision = self.design_vision(project).await?.content;
        let kinds: Vec<String> = sqlx::query_scalar(
            "SELECT json_extract(content_json, '$.kind') FROM design_part
              WHERE project_id = ? AND parent_id IS NULL
                AND json_extract(content_json, '$.kind') IS NOT NULL
              ORDER BY ordinal, id",
        )
        .bind(project.as_str())
        .fetch_all(self.reader())
        .await?;
        let additions = self.current_standards_additions(project).await?;
        let effective = standards::EffectiveStandards {
            base: standards::base().clone(),
            additions,
        };
        Ok(stage::compute(&vision, &kinds, &effective.mandatory_parts()))
    }
}
```

`stage` and `standards` must be `pub(crate)` modules, or the store must import the items through `design`'s re-exports; use whichever matches how `design/store/*.rs` already reach `design` items. Service method in `design/mod.rs`:
```rust
    /// The project's stage (§23.4).
    pub async fn stage(&self, project: &ProjectId) -> Result<StageView, CoreError> {
        Ok(self.storage.project_stage(project).await?)
    }
```
Add the route `get_stage` → `GET /api/projects/{id}/stage` → `StageView`, then re-export `Stage` and `StageView` from `lib.rs`.

- [ ] **Step 5: Run the tests, regenerate, update the contract and code map**

Run:
1. `… cargo test -p shadows-core --test design_parts`
2. `… cargo test -p shadows --test design_routes`
3. `… cargo test -p shadows --test openapi`
4. `cd web && npm run gen:api`
5. `… cargo test -p shadows-core --test contracts`
6. `… cargo test -p shadows --test codemap`

Expected: all pass.

- **Contract:**
  - shapes `Stage`, `StageView`;
  - service `stage`;
  - store `project_stage` (called by Harness);
  - obligation "the stage is computed from the database: idea → vision → map → structure; a mandatory part is a top-level part whose trimmed kind equals its name", tested by the three tests.
- **Code-map rows:**
  - `design/stage.rs` | "a project's stage from its workspace";
  - `design/store/standards.rs` | "versioned standards additions";
  - `design/store/stage.rs` | "reading a project's stage inputs".

- [ ] **Step 6: Commit**

```bash
git add crates api web/src/api/schema.d.ts docs/codebase/README.md
git commit -m "§23.4: the design service computes a project's stage"
```

---

### Task 4: The Planner hears the standards and the stage

**Files:**
- Modify: `crates/shadows-core/src/turns/store/turn.rs` (`NewTurn`, insert, `latest_invocation_versions`)
- Modify: `crates/shadows-core/src/turns/record.rs`, `crates/shadows-core/src/testing/turn.rs`
- Modify: `crates/shadows-core/src/harness/setup.rs`, `crates/shadows-core/src/harness/prompt.txt`
- Modify: `crates/shadows-core/src/turns/spawn.rs` (stage block)
- Modify: `crates/shadows-core/src/design/standards.rs` (`render`)
- Modify: `crates/shadows-core/src/harness/contract.yaml`, `crates/shadows-core/src/turns/contract.yaml`
- Test: `crates/shadows/tests/planner_mcp.rs`, `crates/shadows/tests/plan_in_conversation.rs`

**Interfaces:**
- Consumes:
  - `current_standards_additions` (Task 2);
  - `project_stage`, `stage::line` (Task 3);
  - `base()` (Task 1).
- Produces:
  ```rust
  // turns/store/turn.rs
  pub struct NewTurn<'a> { …, pub standards_version: Option<i64>,
      pub standards_additions_version: Option<&'a str>, … }
  #[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]  // or a tuple read, matching the file
  pub struct InvocationVersions {
      pub prompt: Option<String>,
      pub instructions: Option<String>,
      pub standards: Option<i64>,
      pub additions: Option<String>,
  }
  pub async fn latest_invocation_versions(&self, thread: &ThreadId)
      -> Result<Option<InvocationVersions>, StorageError>
  // design/standards.rs
  pub(crate) fn render(standards: &EffectiveStandards) -> String
  // harness/setup.rs
  pub(crate) async fn stage_line(&self, thread: &ThreadId) -> Result<String, StorageError>
  ```

- [ ] **Step 1: Write the failing tests** in `crates/shadows/tests/planner_mcp.rs`.
  - Add `const STANDARDS: &str = "[Shadows] The project's standards changed";`.
  - Add `const STAGE_IDEA: &str = "[Shadows] Stage: idea.";`.
  - Add a helper `save_additions(app, command, part_name)` that PUTs `/api/projects/{project}/standards/additions` the way `save_instructions` saves.

```rust
#[tokio::test]
async fn a_session_opens_with_the_effective_standards() {
    let l = listening_app().await;
    save_additions(&l.app, "s1", "billing").await;
    let first = report(&l.app).await;
    let append = first["append"].as_str().expect("append");
    assert!(append.contains("## Shadows standards (base v1; project additions v1)"), "{append}");
    assert!(append.contains("- backend (never waived):"), "{append}");
    assert!(append.contains("- billing (project):"), "{append}");
    assert!(append.contains("S1 ["), "{append}");
}

#[tokio::test]
async fn changed_standards_reach_the_next_turn_once() {
    let l = listening_app().await;
    reply(&l.app, "hi").await;
    save_additions(&l.app, "s1", "billing").await;
    let next = report(&l.app).await;
    let sent = blocks(&next);
    assert_eq!(sent.len(), 3, "{sent:?}");
    assert!(sent[1].starts_with(STANDARDS) && sent[1].contains("billing"), "{}", sent[1]);
    assert!(sent[2].starts_with(STAGE_IDEA), "{}", sent[2]);
    let after = report(&l.app).await;
    assert_eq!(blocks(&after).len(), 2, "sent once");
}

#[tokio::test]
async fn a_thread_from_before_the_standards_gets_them_once() {
    let l = listening_app().await;
    reply(&l.app, "hi").await;
    sqlx::query("UPDATE agent_invocation SET standards_version = NULL")
        .execute(l.app.storage.reader())
        .await
        .unwrap();
    let sent = blocks(&report(&l.app).await).len();
    assert_eq!(sent, 3, "standards once");
    assert_eq!(blocks(&report(&l.app).await).len(), 2);
}

#[tokio::test]
async fn every_turn_ends_with_the_stage_line() {
    let l = listening_app().await;
    let first = report(&l.app).await;
    assert_eq!(blocks(&first).len(), 2);
    assert!(blocks(&first)[1].starts_with(STAGE_IDEA));
}
```

How the fake reports `append`: the `report` reply has `"blocks"`. Check `a_session_opens_with_shadows_mcp_instructions_and_the_pre_approval` (line 98) for the key the session's append is read under, and use that key in place of `"append"`.

Update the existing tests for ruling 5. Each list below gains the stage line as its last element:
- `changed_instructions_reach_the_next_turn_once_as_a_context_block`: `sent.len()` 2 → 3; `blocks(&after)` becomes `["report", <stage>]`, asserted with `starts_with(STAGE_IDEA)` on `[1]`.
- `a_thread_from_before_this_milestone_gets_shadows_instructions_once`: the `UPDATE` also sets `standards_version = NULL`, so its `sent[1]` also carries the standards. Assert `sent.len() == 3` (the person's text, then OURS + standards joined in one context block, then the stage). The final `["report"]` becomes length 2.
- `a_new_threads_first_turn_has_no_context_block`: `["report"]` becomes `blocks.len() == 2` with `[1]` the stage line. Rename it to `a_new_threads_first_turn_has_only_the_stage_line`.
- `instructions_saved_after_the_session_opened_reach_its_first_turn`: `len` 2 → 3; the trailing `["report"]` becomes length 2.
- `a_forks_first_turn_gets_instruction_and_continue_plan_blocks`: `len` 3 → 4; `sent[3]` is the stage line.
- `an_invocation_records_the_prompt_and_instructions_versions`: also select `standards_version` and assert `Some(1)`. Destructure `InvocationVersions` where it reads `latest_invocation_versions`.
- `plan_in_conversation.rs`: `["report", told]` becomes `["report", told, stage]`, with `stage` read as `blocks[2]` and asserted with `starts_with("[Shadows] Stage: idea.")`.

- [ ] **Step 2: Run them to see them fail**

Run: `… cargo test -p shadows --test planner_mcp`
Expected: compile error on `InvocationVersions` / `standards_version`, or failures where the stage line is absent.

- [ ] **Step 3: Record the standards on the invocation**
  - **`NewTurn`:** add `standards_version: Option<i64>` and `standards_additions_version: Option<&'a str>`, documented as "the base standards version and the project's current additions id when the turn started (§23.4)".
  - **`start_turn`:** carry both in `versions`. The insert adds the columns `standards_version, standards_additions_version_id` with two more `?` and binds.
  - **`latest_invocation_versions`:** selects the two new columns and maps into `InvocationVersions`. Update its doc.
  - **`turns/record.rs`:** reads `self.storage.current_standards_additions(&context.project_id)` beside the instructions and sets `standards_version: Some(crate::design::base_standards().version)` and `standards_additions_version: additions.as_ref().map(|v| v.id.as_str())`.
  - **`testing/turn.rs`:** sets `standards_version: Some(crate::design::base_standards().version)` and `standards_additions_version: None`.

- [ ] **Step 4: Write `render`** in `standards.rs`

```rust
/// The effective standards as the Planner reads them (§23.4 1–2).
pub(crate) fn render(s: &EffectiveStandards) -> String {
    let additions = s.additions.as_ref();
    let mut out = format!(
        "## Shadows standards (base v{}; project additions {})\n\nMandatory parts:\n",
        s.base.version,
        additions.map_or("none".to_string(), |a| format!("v{}", a.number)),
    );
    for p in &s.base.parts {
        let waiver = if p.waivable { "waivable with a reason" } else { "never waived" };
        out.push_str(&format!("- {} ({waiver}): {}\n", p.name, p.owns));
    }
    for p in additions.iter().flat_map(|a| &a.content.parts) {
        out.push_str(&format!("- {} (project): {}\n", p.name, p.owns));
    }
    out.push_str("\nRules:\n");
    let rules = s.base.rules.iter().chain(additions.iter().flat_map(|a| &a.content.rules));
    for r in rules {
        let parts = if r.parts.is_empty() { "all".into() } else { r.parts.join(", ") };
        out.push_str(&format!("- {} [{parts}]: {}\n", r.id, r.text));
    }
    out.push_str("\nContract template. Rules:\n");
    for r in &s.base.contract_template.rules {
        out.push_str(&format!("- {r}\n"));
    }
    out.push_str(&format!(
        "Sections, in order: {}\n",
        s.base.contract_template.shape.join(", ")
    ));
    out
}
```

- [ ] **Step 5: Update `harness/setup.rs`**
  - **`Held`:** gains `standards: Option<String>`, the additions id its append carried.
  - **`for_opening`:** reads `current_standards_additions(project)`, builds `EffectiveStandards { base: base_standards().clone(), additions }`, and puts `render(&effective)` in the append after Shadows' instructions and before `## Project instructions`:
    ```rust
    let mut append = format!("{}\n\n{}", ours(), render(&effective));
    if let Some(body) = body(current.as_ref()) {
        append.push_str(&format!("\n\n## Project instructions\n\n{body}"));
    }
    ```
  - **`context_before_turn`:**
    - Read the additions and `latest` as `InvocationVersions`.
    - Compute `standards_sent: bool`:
      - with an invocation: `latest.standards == Some(base_standards().version) && latest.additions == current_additions_id`;
      - fork with none: `false`;
      - fresh thread: compare `Held.standards` with the current id.
    - When false, push:
      ```rust
      const STANDARDS: &str = "[Shadows] The project's standards changed. \
                               They replace the standards you were given before:";
      parts.push(format!("{STANDARDS}\n\n{}", render(&effective)));
      ```
      after the instructions part.
  - **New method:**
    ```rust
    /// §23.4 3: the line every turn carries, naming the stage and what is missing.
    pub(crate) async fn stage_line(&self, thread: &ThreadId) -> Result<String, StorageError> {
        let context = self.storage.turn_context(thread).await?;
        let view = self.storage.project_stage(&context.project_id).await?;
        Ok(crate::design::stage_line(&view))
    }
    ```
    Export `stage::line` from `design` as `pub(crate) use stage::line as stage_line;`.
  - **Module doc:** add "and the standards (§23.4)".

- [ ] **Step 6: Send the stage line last** in `turns/spawn.rs`. Next to `context_before_turn`, read `let stage = sessions.setups().stage_line(&thread_id).await;` and fold it into `prepared`:
```rust
        let prepared = match (context, stage) {
            (Ok(context), Ok(stage)) => {
                (sessions.prepare_turn(&thread_id, &opened, &settings).await).map(|()| {
                    context
                        .into_iter()
                        .chain(focus)
                        .chain(continue_plan)
                        .chain(std::iter::once(stage))
                        .collect::<Vec<String>>()
                })
            }
            (Err(error), _) | (_, Err(error)) => Err(error.to_string()),
        };
```

- [ ] **Step 7: Tell the Planner** in `prompt.txt`. Add a section after "Shadows' tools":
```text
Standards and stage
- Shadows' standards for this project are below, after these instructions, and Shadows tells you before a message when they change. They are not suggestions: every project has the mandatory parts, each part has a contract written to the template, and no part does another's job.
- When the person asks how to build something, propose it as these parts — a separate backend and database at the least — never a single file or a backend mixed with its database, however small the idea.
- Each of the person's messages ends with a line from Shadows naming the project's stage and what is missing. Work at that stage: discuss the idea at "idea", complete the missing vision fields one at a time at "vision", and propose the missing parts at "map". Do not skip ahead.
```

- [ ] **Step 8: Run the tests**

Run:
1. `… cargo test -p shadows --test planner_mcp`
2. `… cargo test -p shadows --test plan_in_conversation`
3. `… cargo test -p shadows --test planner_turn`
4. `… cargo test -p shadows-core --test sessions`

Expected: all pass. A failure in another test that compares a turn's exact blocks gets the same edit as Step 1: the stage line, last. Record it in the ledger.

- [ ] **Step 9: Update the contracts**
  - **`harness/contract.yaml`:**
    - `stage_line`;
    - the standards in `for_opening` and `context_before_turn`;
    - obligations, each naming its test from Step 1:
      - "standards reach a session at open and once after each change, never mid-turn";
      - "every turn ends with the stage line".
  - **`turns/contract.yaml`:** the invocation records the standards versions; `InvocationVersions`.

Run `… cargo test -p shadows-core --test contracts`.

- [ ] **Step 10: Commit**

```bash
git add crates
git commit -m "§23.4: the Planner hears the standards at open, on change, and the stage each turn"
```

---

### Task 5: Web — Standards tab and the stage in the header

**Files:**
- Modify: `web/src/api/client.ts` (types + `getStandards`, `saveStandardsAdditions`, `getStage`)
- Modify: `web/src/api/queries.ts` (`standardsQuery`, `stageQuery`)
- Create: `web/src/app/design/standards-view.tsx`
- Create: `web/src/app/design/standards-view.test.tsx`
- Create: `web/src/app/conversation/stage-chip.tsx`
- Modify: `web/src/app/design/workspace-page.tsx` (tab + view)
- Modify: `web/src/router.tsx` (`view` search value `'standards'`, if the search is validated there)
- Modify: `web/src/app/conversation/conversation.tsx` (chip in the header)
- Modify: `web/src/stream/use-project-events.ts` (+ its test)
- Modify: `web/src/test/fake-daemon.ts` (serve the three routes)

**Interfaces:**
- Consumes: `GET /api/projects/{id}/standards`, `PUT …/standards/additions`, `GET /api/projects/{id}/stage` (Tasks 2–3), with types from `schema.d.ts`.
- Produces:
  - `standardsQuery(projectId)` with key `['projects', projectId, 'design', 'standards']`;
  - `stageQuery(projectId)` with key `['projects', projectId, 'design', 'stage']`.
  - Both sit under `'design'`, so the existing `ProjectDesignChanged` invalidation refreshes the stage.

- [ ] **Step 1: Write the failing tests**
  - **`standards-view.test.tsx`:** follow `vision-editor.test.tsx` for render and fake-daemon setup.
    - The base parts render with "never waived" for backend.
    - Rule `S1`'s text renders, with no edit control for base items.
    - Adding a project part `billing` with "Payments." and pressing Save PUTs `{ command_id, content: { rules: [], parts: [{ name: 'billing', owns: 'Payments.' }] } }`, then shows "version 1".
    - A refusal's message shows through `ErrorLine`.
  - **`use-project-events.test.tsx`:** a `ProjectStandardsSaved` durable event invalidates `['projects', 'p1', 'design']`. Mirror the existing `ProjectDesignChanged` case at line 37.
  - **`conversation.test.tsx`:** with the fake daemon answering `{ stage: 'map', missing: ['database'] }`, the header shows `Stage: map` and `missing database`.

- [ ] **Step 2: Run them to see them fail**

Run: `cd web && npx vitest run src/app/design/standards-view.test.tsx src/stream/use-project-events.test.tsx src/app/conversation/conversation.test.tsx`
Expected: FAIL. The module `./standards-view` is not found, and the other cases fail on their assertions.

- [ ] **Step 3: Implement**
  - **Client and queries:** add the functions next to `getInstructions`/`saveInstructions` in `client.ts`, and the queries next to `instructionsQuery` in `queries.ts`. Use the generated schema types `EffectiveStandards`, `StandardsAdditions`, `StandardsAdditionsVersion` and `StageView`.
  - **`standards-view.tsx`:**
    - Header comment: `// One job: the project's standards (§23.2): Shadows' base, read only, and the project's additions, edited and saved as a new version.`
    - Sections, top to bottom:
      - Mandatory parts: name, owns, "never waived" or "waivable with a reason";
      - Rules: id, parts, text;
      - Contract template: rules as a list, shape as a comma list;
      - Project additions.
    - The Project additions editor:
      - one row per added part, with name and owns inputs and Remove;
      - one row per added rule, with id shown, a text textarea and a parts input taking a comma list;
      - "Add part" and "Add rule" buttons; a new rule gets the next free `P<n>`;
      - Save uses `attemptFor`/`useMutation` exactly as `instructions-editor.tsx` does: same pending-command reuse, and `setQueryData` on success.
    - Every user-text element gets `dir="auto"`.
  - **`workspace-page.tsx`:** add `<Link … search={{ view: 'standards' }}>Standards</Link>` after "Project map", and `view === 'standards' ? <StandardsView key={projectId} projectId={projectId} />` in the chain. If `router.tsx` validates `view` against a list, add `'standards'`.
  - **`stage-chip.tsx`:**
    - Header comment: `// One job: the project's stage (§23.4) as one line in the conversation header.`
    - It uses `stageQuery`.
    - It renders `Stage: {stage}`, plus ` · missing {missing.join(', ')}` when `missing` is non-empty.
    - Classes: `text-xs text-muted-foreground`.
    - It renders nothing while loading or on error.
  - **`conversation.tsx`:** place `<StageChip projectId={…} />` in the header next to the title, using the project id the pane already has.
  - **`use-project-events.ts`:** treat `ProjectStandardsSaved` exactly like `ProjectDesignChanged`: `if (event.kind === 'ProjectDesignChanged' || event.kind === 'ProjectStandardsSaved')`.
  - **`fake-daemon.ts`:** answer the three routes, with a base fixture of the four parts and one rule `S1`, and `{ stage: 'idea', missing: [] }` by default, overridable per test the way other routes are.

- [ ] **Step 4: Run the tests to see them pass**

Run:
1. The Step 2 command;
2. `cd web && npx tsc -b --noEmit` (or `npm run build` if `tsc -b` is not the project's script);
3. `npm run lint`.

Expected: all pass, with no type or lint errors.

- [ ] **Step 5: Commit**

```bash
git add web
git commit -m "§23.2, §23.4 web: a Standards tab and the stage in the conversation header"
```

---

### Task 6: Spec, status, browser run, gate

**Files:**
- Modify: `docs/superpowers/specs/2026-10-08-guided-planning-design.md`
  - §23.2: "edited from the project's settings" becomes "edited from the workspace's Standards tab".
  - §23.4: add one sentence: "Drift arrives with vision items (PR 2) and map approval (PR 3); until then the stage reports what is missing only."
- Modify: `docs/status.md` (§23 PR 1 line)

- [ ] **Step 1: Amend the spec and status as listed.**

- [ ] **Step 2: Browser run on a copy of the development database** (memory: verification habits).
  1. Copy the development database to `run-24.sqlite3`.
  2. Point `.claude/launch.json` at the copy.
  3. Start the daemon and web preview with `preview_start`.
  4. In a new project, open the workspace's Standards tab.
     - Check: the base standards show.
     - Add part `billing`, then save.
     - Check: version 1.
  5. Open a conversation.
     - Check: the header shows `Stage: idea`.
     - Send: "I want a small app to sell used books; how do we build it?"
     - Check: the Planner's reply proposes separate parts, at least a backend separate from the database, not a single file.
  6. Fill the vision on the Vision tab.
     - Check: the header changes to `Stage: map · missing backend, database, api, frontend, billing` without a reload.
  7. Check the daemon logs for errors.
  8. Restore `.claude/launch.json` and delete `run-24.sqlite3`.

  Expected: each check holds. Record what the Planner proposed in the ledger.

- [ ] **Step 3: Full gate, once**

Run, from the root, with the cargo prefix:
1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings`
3. `cargo test --workspace`
4. `cargo clippy --workspace -- -D warnings`
5. `cargo tree -e features,no-dev --workspace | grep test-support`
6. `git diff --exit-code api/`
7. In `web/`: `npm test`, `npm run lint`, `npm run build`
8. Check: no Rust line over 100 columns in the diff.

Expected:
- every step passes;
- step 5 prints nothing, so its exit 1 is the expected result;
- step 6 shows no diff once committed.

- [ ] **Step 4: Commit**

```bash
git add docs
git commit -m "§23 PR 1: standards and stage built and run; §23.2 and §23.4 amended"
```
