# ResearchArtifact is durable from v1

**Doc ID:** 984
**Status:** accepted
**Tags:** architecture, continuity, foundation, research
**Source slug:** researchartifact-is-durable-from-v1

---

# Decision — ResearchArtifact is durable from v1

**Status:** accepted · 2026-09-20

## Rule

Research artifacts are durable from v1, even though the research
subsystem is minimal. We do not defer the concept itself.

## Why

Continuity requires it:

```text
Claude researches architecture
→ tomorrow Codex continues planning
→ Codex must know what was researched and what evidence existed
```

Without durable artifacts, the second agent has no ground truth.

## v1 model (minimal)

```rust
struct ResearchArtifact {
    id,
    project_id,
    title,
    source?,
    summary,
    created_at,
}
```

NO search engine. NO embeddings. NO crawler. Just enough that a
future agent can read what was researched and what was concluded.

## Deferred (explicitly)

- Full-text search across artifacts
- Embeddings / similarity search
- Automated crawlers / fetchers
- Cross-artifact linking graphs

Related: [[shadows-owns-truth-not-the-cli]], [[continuity-is-cross-module-invariant]]
