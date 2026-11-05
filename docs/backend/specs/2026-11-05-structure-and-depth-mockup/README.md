# Structure and depth mockup

Source files of the mockup for
[the structure and depth design](../2026-11-05-structure-and-depth-design.md).
The published copy is https://claude.ai/artifact/No9kfXkwK12wctXundandy.
These files keep it if the link is lost. Each `.dc.html` is one screen,
and `canvas.json` lays the screens out.

| File | Screen |
|---|---|
| `Main.dc.html` | 1. Project map: parts, dependency arrows, planning depth panel |
| `Part.dc.html` | 2. Part view: the backend's layers as rings, with modules and ports |
| `Layer.dc.html` | 3. Layer view: database schema as an ER diagram, element panel and trace |
| `Plan.dc.html` | 4. Task view: T3 opened from the Stage 1 graph, its phases drawn as the plan graph is |

The data is the testing56 support system. Mohammed approved the task view
on 2026-11-05.

## Review amendments, 2026-11-05

These four source screens retain the original proposal. The revised design
§6 owns the implementation behavior. The task graph and Show phases direction
remain, with these corrections required before a frontend task is dispatched.

| Screen | Required correction |
|---|---|
| Main | Each part shows its summary sentence ("8 described, 5 specified, 3 need decisions"), not a bar or percentage. Remove `3 Built`. Database has schema, migrations and connection only. Actual dependencies and allowed catalog edges need separate legends. Outcome coverage lives on Roadmap, not here. |
| Part | Keep a keyboard-accessible layer list alongside conceptual rings. Interface and infrastructure are peers. Show module filters, applicable roles and recorded-design validation separately from code checks. |
| Layer | A table has a logical schema location and an implementation link to its migration. The migration's `up`/`down` methods belong to the migration element. Show approved revision, declared trace direction and unknown links. |
| Plan | Show Draft/Approved and execution availability separately. Replace `0 done` and `starts now` claims without execution evidence. Pin foreign phase versions. Preserve whole-task constraints and tasks without phases when expanded. |

The Roadmap coverage view is specified in design §§5.5.1 and 6.1 and still
needs a UI prototype before its task is dispatched. It shows acceptance rows,
linked work, missing decisions and evidence with one next action per blocker.
The sketches above are fixed-size layouts, so they do not establish narrow
screen, keyboard or real-app acceptance. Design §6.3 supplies those checks.

The saved HTML references `support.js`/`DCLogic`, which are not included in
this directory. A rendered prototype must first supply the intended preview
runtime or remove that export dependency. Static source inspection on
2026-11-05 was possible; the in-app browser reported local browser use disabled.
No visual acceptance is recorded by this review.
