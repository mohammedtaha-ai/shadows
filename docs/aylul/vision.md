# NYLRA - Vision

## Human Liaison for Agentic Systems

**Product:** A standalone AI companion and human-liaison platform that connects a person to complex multi-agent systems such as Shadows. AYLUL is its companion character.

**Updated:** 2026-10-19. This file holds the full product idea. It records
intended capabilities, not implemented software, and decides nothing: each
decision lives in its owner design under `docs/aylul/`.

## Current direction (2026-10-19)

AYLUL is Mohammed's assistant. It stands between him and his agents (Claude,
Codex, Gemini, and Shadows' planner and executors), explains their work in his
language, carries his decisions back, and later handles personal tasks such as
email and music. It must keep working when one agent's subscription runs out.

- **Brain:** Claude Code in AYLUL's own home folder, with its own profiles,
  provider and key, run by Shadows core inside the same product daemon and
  database. Hermes is no longer the brain. Owner:
  [brain design](specs/2026-10-19-aylul-claude-code-brain-design.md).
- **Screen:** the Shadows web client and its settings, with AYLUL as its own
  section. The character stays a separate native window.
- **Where it works:** from its own home, never inside a project. It learns about
  work through hooks and reads what it needs when asked.
- **Order:** brain; work profile (watching agents, explaining, alerting);
  personal profile (email, YouTube, preferences); voice. Remote alerts (for
  example Telegram) later.
- **Shadows flow it serves later:** the planner hands tasks to executors; an
  executor fixes what is inside its task and reports the rest to the planner;
  the planner adjusts or escalates to Mohammed; when he is away, AYLUL relays
  the question and his answer.
- **Later, not planned yet (Mohammed, 2026-10-21):** Core gives each AYLUL agent its own
  MCP tools for AYLUL's screen, the way the Claude desktop app gives Claude Code its tools
  for the task list and the next-step box: for example, set the character's state, or show
  a task list in the conversation. It gets its own spec and plan when it is picked up.

The sections below are the original long-form vision. Where they name Hermes,
a cloud brain, or a separate NYLRA backend, the direction above replaces them.

---

## 1. Executive Summary

Modern AI agent systems are becoming increasingly capable, but the human experience of supervising them is still primitive.

A planner may discover an architectural blocker. An executor may fail because a dependency changed. A critic may reject a plan. A research agent may discover evidence that invalidates yesterday's decision. Today, the human owner often has to open a dashboard, inspect transcripts, read logs, re-load context, understand technical details, ask follow-up questions, and manually relay decisions back to the planner.

That does not scale.

The vision is to build a **standalone AI Human Liaison**: a personal, context-aware, voice-capable assistant that sits between the human and an agentic work system.

Its job is not to plan, code, execute, or silently make decisions.

Its job is to:

- understand what the agents are doing,
- retrieve the relevant project context,
- explain problems in the human's language,
- remember how the human prefers to make decisions,
- ask the human only when needed,
- carry the human's proposed direction back to the planner for technical evaluation,
- return the planner's opinion in a concise form,
- obtain explicit human approval when required,
- deliver the approved decision back to the system,
- and keep the person informed without forcing them to live inside agent transcripts.

The first integration target is **Shadows**, but the product should be designed as an independent platform that could eventually integrate with other agent frameworks and orchestration systems.

The key idea is simple:

> The human should not have to become another agent in the workflow.

The Liaison becomes the human-facing interface to the entire agent organization.

---

## 2. The Product in One Sentence

**A personal AI Chief of Staff that understands your agent systems, explains what matters, asks for your decisions, carries your intent back to the planners, and keeps you connected through desktop, mobile, text, voice, notifications, and calls.**

---

## 3. The Problem

Agentic systems increasingly contain multiple specialized roles:

- planners,
- executors,
- critics,
- researchers,
- architecture agents,
- code agents,
- test agents,
- deployment agents,
- project managers,
- and higher-level executive agents.

The more capable the system becomes, the harder it becomes for the human owner to supervise it.

The core problems are:

### 3.1 Context overload

A planner may understand a problem because it has loaded project plans, contracts, decision history, repository structure, and recent events.

The human may see only:

> "Planning blocked: storage architecture conflicts with scheduler requirements."

To understand that sentence, the person may have to open several files and reconstruct previous decisions.

### 3.2 Token waste

Expensive planning models should spend their context and reasoning budget on planning and architecture, not repeatedly re-explaining historical context to the user.

A planner should be able to emit a compact structured blocker such as:

```text
BLOCKER B-218

Decision D-71 conflicts with scheduler requirement SR-12.

Candidate directions:
A. Revise persistence architecture.
B. Preserve SQLite and introduce an abstraction layer.

Human decision required.
```

The Liaison should resolve the references, read the relevant decisions and plans, and explain the issue conversationally.

### 3.3 Human attention does not scale

A person should not need to monitor dashboards continuously.

Most events are not important enough to interrupt them.

The system needs to know the difference between:

- informational progress,
- warnings,
- decisions that can wait,
- important architectural questions,
- and critical blockers that justify a phone call.

### 3.4 Agent output is not human communication

Agents are optimized for technical work.

Humans often want answers such as:

- "What actually went wrong?"
- "Why does this matter?"
- "Didn't we already decide this yesterday?"
- "What happens if we ignore it?"
- "What does Claude recommend?"
- "Will this break any approved plan?"
- "Can this wait until tomorrow?"

The Liaison translates machine workflow state into human decision context.

### 3.5 Remote supervision is weak

The owner may be away from the workstation.

They should still be able to:

- receive a concise notification,
- hear a voice briefing,
- ask follow-up questions,
- request the planner's opinion,
- approve or reject a direction,
- pause work,
- and resume execution later.

This is why the product should not be treated as a small feature inside a local dashboard.

It deserves its own architecture.

---

## 4. The Core Vision

The system should feel like having a highly competent personal technical representative.

The representative understands the projects, understands the agents, understands the owner's preferences, but does not impersonate the owner.

A typical interaction should work like this:

```text
Claude Planner
    |
    | "We found a conflict between the approved storage design
    |  and the shared scheduler requirement."
    v
Human Liaison
    |
    | Retrieves:
    | - previous decision
    | - affected plans
    | - relevant design notes
    | - current blocker
    |
    | Explains the issue to the human
    v
Human
    |
    | "Keep SQLite for now.
    |  Add an abstraction layer.
    |  But ask Claude whether that causes problems later."
    v
Human Liaison
    |
    | Sends a proposal, not an approval
    v
Claude Planner
    |
    | Evaluates viability, risks, and downstream effects
    v
Human Liaison
    |
    | "Claude agrees. It adds a small scheduler refactor later,
    |  but it does not block any approved plan."
    v
Human
    |
    | "Okay, approve it."
    v
Decision Gateway
    |
    | Records explicit human confirmation
    v
Claude Planner
    |
    | Applies the accepted direction through Shadows' service rules
    | Proposes or updates the appropriate plan version
    | Continues work
```

The Liaison is the communication layer.

The Planner remains the planning authority.

The human remains the decision authority.

Shadows records acceptance against the exact current proposal before NYLRA
reports it adopted. An assessment is Planner's opinion; delivery alone does
not prove that a decision was accepted or that execution resumed.

---

## 5. Relationship With Shadows

NYLRA's desktop and mobile applications live in the **separate `nylra` repository**.
Its cloud application logic lives in the shared **`shadows-backend` repository**.

Shadows should remain a local-first orchestration platform.

The Liaison should be an optional connected service.

The chosen experience is **personal work on its owning computer**, **team work
owned by its server**, and **NYLRA conversation following the person across
devices**. A laptop may execute team assignments without becoming a second owner.
Personal projects and preferences are not exposed through team membership.

Sign-in does not upload projects. Linking is not backup, team publication, code
sync or execution transfer. An offline personal computer cannot run a new Planner
turn; retained explanations are visibly cached. The authoritative decisions are in
[Shadows §1.7](../superpowers/specs/2026-09-21-architecture-design.md#17-workspace-authority-and-deployment),
with the [sharing inventory](../../../shadows-backend/docs/architecture.md#what-lives-where)
specifying what crosses into the cloud.

### 5.1 Why separation matters

The two products have different responsibilities.

**Shadows** owns:

- projects,
- plans,
- dependencies,
- contracts,
- agents,
- execution,
- decisions,
- events,
- audit trails,
- project state,
- orchestration.

**The Liaison** owns:

- human communication,
- personal preferences,
- voice interaction,
- mobile presence,
- device synchronization,
- notification policy,
- conversation history,
- provider configuration,
- remote access,
- personal memory,
- human-facing summaries.

These responsibilities retain separate ownership across the product.
Project rules remain in Rust for both local and future server deployments;
the TypeScript application calls them rather than implementing another copy.
An application process may host several domains without allowing them to
bypass each other's public operations or private stores.

### 5.2 SQLite should not be abandoned because of the assistant

Shadows can remain local-first and continue using SQLite where it is appropriate.

The Liaison has different requirements.

A mobile-connected assistant may need:

- account identity,
- secure pairing,
- push notifications,
- device registration,
- remote event delivery,
- encrypted sync,
- presence,
- call orchestration,
- multi-device state.

Those concerns justify a server or relay service.

They do **not** justify converting Shadows itself into a server-centric application.

### 5.3 The assistant must never depend on Shadows' database internals

The Liaison should not read Shadows' SQLite database directly.

It should communicate through a formal, versioned protocol.

Illustrative future operations, not implemented API names:

```text
get_project_status()
get_plan()
get_decision()
get_blocker()
get_agent_status()
get_related_context()

request_planner_assessment()
submit_user_response()
submit_human_decision()
pause_work()
resume_work()
```

This boundary is critical.

If Shadows changes its internal storage later, the Liaison should not care.

---

## 6. High-Level Architecture

The [shared architecture sections 2-4](../../../shadows-backend/docs/architecture.md#2-languages-and-technology-direction)
own the selected languages and connection topology. Desktop can reach local
Shadows for navigation and presentation, and the cloud for NYLRA dialogue.
Mobile reaches the cloud. Shadows maintains its own outbound connection even
when the NYLRA desktop interface is closed.

NYLRA's reasoning orchestration, personal memory, and topic conversations are
cloud-hosted. Native desktop functions belong to Tauri's Rust side; the mobile
direction is Flutter/Dart. Exact libraries still require capability evidence.

The work escalation sequence is:

```text
Executor -> Manager -> Planner -> NYLRA -> Human
```

Each role settles what its authority covers before escalation. NYLRA becomes
involved when the person is needed; it is not an intermediate manager. The
Manager and Executor roles remain future Shadows implementation work.

---

## 7. The Liaison Is Not the Planner

This distinction must remain explicit in the product architecture.

### The Planner

The Planner:

- reasons about architecture,
- decomposes work,
- evaluates technical tradeoffs,
- manages dependencies,
- proposes implementation plans,
- reacts to project constraints,
- records formal project decisions when authorized.

### The Liaison

The Liaison:

- reads and retrieves context,
- explains,
- summarizes,
- asks questions,
- remembers user preferences,
- transports the user's intent,
- requests planner evaluation,
- returns the planner's opinion,
- confirms approval,
- communicates state.

It should not silently take over planning authority.

### The Executor

The Executor:

- changes files,
- runs commands,
- modifies code,
- executes approved work.

The Liaison should not become an executor.

---

## 8. Permission Philosophy

The Liaison should be powerful in understanding and communication, but deliberately limited in execution.

A preferred permission model is:

```text
READ:
- project status
- plans
- blockers
- decisions
- relevant artifacts
- agent status
- approval requests
- audit history

COMMUNICATE:
- send message to planner
- request planner opinion
- relay user intent
- request clarification
- request alternative options

CONTROL:
- request pause
- request resume
- submit explicit human approval
- submit explicit human rejection
```

The Liaison should not receive unrestricted abilities such as:

```text
edit_code()
rewrite_plan_directly()
approve_without_human()
delete_project()
change_architecture()
execute_shell()
```

If future features require stronger powers, they should be opt-in, scoped, auditable, and policy-controlled.

---

## 9. Human-in-the-Loop Decision Gateway

One of the most important architectural components should be a dedicated **Decision Gateway**.

The Gateway exists to preserve the distinction between:

- assistant suggestion,
- planner recommendation,
- user proposal,
- and human approval.

A proposal might look like:

```text
PROPOSED_DIRECTION

Source: Human
Intent:
- Keep SQLite.
- Add a storage abstraction.
- Revisit distributed scheduling later.

Planner action:
Evaluate only.

Do not adopt yet.
```

The planner might answer:

```text
PLANNER_ASSESSMENT

Viable: yes
Risk: moderate-low
Current approved plans blocked: none
Future cost: scheduler abstraction refactor
Recommendation: proceed
```

The Liaison then asks the human:

> Claude considers the direction safe. It will create a small refactor later, but no current approved plan is blocked. Approve?

Only after the user confirms does the Gateway emit:

```text
HUMAN_DECISION

Decision: approved
Scope: project X
Proposal: P-184
Confirmed by: human
Channel: mobile voice
Timestamp: ...
```

This event can then be accepted by Shadows.

This conceptual record is not sufficient proof by itself. The trusted client
binds a deliberate confirmation to the exact displayed operation/proposal.
[Shadows §24.6](../superpowers/specs/2026-10-08-approval-authority-design.md#246-external-human-confirmation-and-assessment)
owns enforcement, Planner-versus-human attribution and assessment write limits.
Voice prepares the response; voice-only confirmation needs separately reviewed
proof before it can replace the deliberate client confirmation action.

---

## 10. Two Kinds of Memory

The system needs two fundamentally different memory domains.

### 10.1 Personal memory

Personal memory helps the Liaison understand the human.

Examples:

```text
- Prefers architectural changes to require explicit approval.
- Usually wants the Planner's opinion before accepting major design changes.
- Prefers short voice summaries unless more detail is requested.
- Does not want non-critical calls after 11 PM.
```

This is not project truth.

It is interaction context.

### 10.2 Project memory

Project memory is authoritative project state.

Examples:

```text
Decision D-71
Approved by: Human
Subject: Persistence architecture
Decision:
Keep SQLite as workspace storage for Stage 1.

Reason:
Local-first simplicity and operational reliability.
```

The Liaison may use personal memory as a hint.

It should use project records as evidence.

### 10.3 Memory should never replace provenance

If the user asks:

> "Didn't we decide yesterday that we would keep SQLite?"

The Liaison should not answer from vague conversational recollection.

It should:

```text
Memory hint
    |
    v
Search project decisions
    |
    v
Find D-71
    |
    v
Verify
    |
    v
Answer with source
```

This creates trustworthy memory rather than confident guessing.

### 10.4 Working context is a separate scope

Each ongoing topic names its project, affected work, relevant Planner thread,
and versioned evidence. That association is not inferred from whichever
conversation happened to be newest globally. The companion can retrieve a
relevant older decision without copying the complete conversation history.
The sharing rules are owned by
[shared architecture section 6](../../../shadows-backend/docs/architecture.md#6-current-work-context-not-transcript-replication).

---

## 11. Context Engine

The Context Engine is one of the most important parts of the system.

Its job is to convert structured agent events into useful human explanations.

For selected linked projects, the companion obtains the bounded current work
context and sources needed for a topic. This also covers opening NYLRA after
talking directly with Planner. It is not a full Planner transcript replica.

Given a blocker, it may retrieve:

- the current plan,
- the previous approved decision,
- affected projects,
- dependency relationships,
- recent planner messages,
- contract changes,
- failed validation,
- relevant code references,
- risk notes.

Then it produces a concise explanation.

The Liaison should be able to answer:

- "What is Claude referring to?"
- "Show me the decision we made before."
- "Why does this conflict exist?"
- "What breaks if we choose option B?"
- "Is this urgent?"
- "Which projects are affected?"
- "What does the planner recommend?"
- "Why did the planner change its opinion?"
- "Can we postpone this decision?"

---

## 12. Token Efficiency as a Product Feature

The Liaison should reduce expensive model usage, not add another layer of waste.

The Planner should not repeatedly reconstruct human-facing explanations.

A more efficient architecture is:

```text
Planner:
Produces compact structured event
        |
        v
Liaison:
Retrieves relevant context
        |
        v
Human:
Gets explanation
```

Instead of:

```text
Planner:
Loads huge history
Explains everything again
Answers user
Loads more history
Re-explains
```

The system should track:

- tokens used by Planner,
- tokens used by Liaison,
- context retrieval cost,
- compression savings,
- repeated-context avoidance.

Eventually, the Liaison can make token economics visible:

> This explanation used 1,200 Liaison tokens instead of reloading approximately 18,000 tokens of Planner context.

Token efficiency can become a measurable product capability.

The objective is to route repeated clarification and voice dialogue to
NYLRA's separately selected, cheaper model, reserving Planner calls for
technical assessment and planning. The figures above are illustrative, not
measured savings. Include retrieval, speech, retries, and correction costs in
any comparison. More dialogue with NYLRA need not consume another Planner turn.

---

## 13. Provider Abstraction

The user should be able to choose the AI provider.

The first model should be **BYOK: Bring Your Own Key**.

Possible reasoning providers may include:

- OpenAI,
- Anthropic,
- Google,
- OpenRouter-compatible providers,
- local models,
- enterprise-hosted models,
- future providers.

This inventory does not mean cloud NYLRA can reach a local model. Initial providers
must be reachable by the cloud application. Local reasoning/speech providers need
a separately scoped execution design.

The system should not assume one model is best for all tasks.

A user may choose:

```text
Reasoning:
Claude / GPT / Gemini / local model

Speech-to-Text:
provider A

Text-to-Speech:
provider B

Fast summarization:
small low-cost model

Critical reasoning:
strong premium model
```

This separation creates flexibility and cost control.

---

## 14. Voice Must Be Decoupled From Reasoning

Voice support should not require the main LLM provider to support audio.

The architecture should be:

```text
Speech Input
    |
    v
Speech-to-Text
    |
    v
Reasoning LLM
    |
    v
Text-to-Speech
    |
    v
Voice Output
```

This allows:

- a strong text-only reasoning model,
- a cheap or free-compatible STT option,
- a different high-quality TTS provider,
- local speech models,
- or text-only mode.

If speech services are unavailable, the assistant should degrade gracefully:

```text
Voice unavailable
      |
      v
Text mode
```

Voice is a channel, not the intelligence itself.

---

## 15. Channel Adapters

The core assistant should be independent from the communication channel.

Potential adapters:

- mobile app,
- desktop companion,
- dashboard chat,
- push notification,
- voice conversation,
- phone call,
- email summary,
- wearable notification,
- future automotive integration,
- future messaging integrations.

The same decision should be reachable through different surfaces.

Example:

```text
Blocker B-218
    |
    +--> Push notification
    |
    +--> Mobile chat
    |
    +--> Voice briefing
    |
    +--> Phone call if critical
```

---

## 16. Notification and Escalation Policy

The assistant should not interrupt the user for everything.

A policy engine should classify events.

Example:

```text
Low severity
    -> log only

Normal progress
    -> grouped summary

Important issue
    -> push notification

Decision required
    -> interactive notification

High-impact blocker
    -> voice alert

Critical blocker
    -> phone call
```

The user should be able to define policies such as:

> Do not call me unless a project has been blocked for more than 20 minutes or a decision affects architecture, security, money, or production.

Policies may include:

- severity,
- project,
- working hours,
- quiet hours,
- business impact,
- deadline proximity,
- confidence,
- number of affected plans,
- financial cost,
- risk level.

---

## 17. Mobile Experience

The mobile application is not just a remote dashboard.

It should be a human decision interface.

Example notification:

> **Shadows needs your decision**
>
> Claude Planner found a conflict in Backend Architecture.
> I can explain it in about one minute.

Actions:

- Explain
- Ask Planner
- Approve
- Reject
- Remind Me Later
- Open Details

Voice flow:

> Human: "Explain it."

> Liaison: "Yesterday you approved SQLite for workspace storage. The new shared scheduler requirement assumes shared coordination across workspaces. Claude sees two possible paths..."

> Human: "Keep SQLite and add an abstraction, but ask Claude if that creates problems later."

> Liaison: "I'll ask for a technical assessment. I will not approve anything yet."

Later:

> Liaison: "Claude agrees. It expects a small refactor when distributed scheduling is introduced, but no current plan is blocked. Approve?"

> Human: "Yes."

The person confirms the exact proposal in the trusted client surface. The model
does not convert the transcribed "Yes" into approval evidence on its own.

> Liaison: "Your confirmation is queued."

Only after the corresponding owner outcomes arrive:

> Liaison: "Shadows accepted the decision. Planning has resumed."

That is the target experience.

---

## 18. Desktop Companion

The desktop version can take inspiration from the category of ambient desktop companions.

It may have:

- a small always-available presence,
- a menu-bar or tray mode,
- optional animated avatar,
- voice activation,
- glanceable project state,
- live agent status,
- quick approvals,
- drag-and-drop context,
- short spoken briefings.

The presence may feel warm and personal without becoming distracting.

The visual identity, character, sounds, and animations should be original.

Projects such as Coucou may inspire interaction patterns, but branding and protected media should never be copied.

---

## 19. Product Activation Flow

Shadows should work without the assistant.

The assistant is optional.

A possible activation flow:

```text
Shadows Dashboard
      |
      v
Enable Assistant
      |
      v
Create account / Sign in
      |
      v
Pair this Shadows instance
      |
      v
Select projects to enable for NYLRA
      |
      v
Choose AI provider
      |
      v
Add API key
      |
      v
Optional speech providers
      |
      v
Install mobile or desktop companion
      |
      v
Connected
```

The user should be able to disable the assistant without affecting Shadows.

---

## 20. Pairing Model

Each Shadows installation should have a distinct identity.

Example:

```text
Account
 |
 +-- Laptop
 |    |
 |    +-- Shadows Instance A
 |
 +-- Workstation
 |    |
 |    +-- Shadows Instance B
 |
 +-- Server
      |
      +-- Shadows Instance C
```

The mobile app may connect to all three through one user account.

Permissions must remain scoped.

Pairing an instance does not publish all its projects. Linking is enabled
per project. One companion can serve several linked projects and instances,
with each topic and its sources isolated to the permitted scope. Account-wide
communication preferences do not grant cross-project access or approval power.
Stable account identifiers bind ownership; name and email are attributes.

The assistant should know:

- which instance generated an event,
- which projects belong to that instance,
- which commands are allowed,
- which device was used for approval.

---

## 21. Secure Relay Architecture

The local Shadows instance should not need to expose a public web server.

A connector owned by the local Shadows daemon creates an outbound authenticated
connection to the shared backend. Its lifetime does not depend on a NYLRA window:

```text
Shadows
    |
    | outbound secure WebSocket
    v
shadows-backend
    |
    v
Mobile / Desktop
```

Benefits:

- NAT-friendly,
- firewall-friendly,
- no manual port forwarding,
- simpler mobile connectivity,
- revocable device sessions,
- easier push integration.

The relay should minimize retained content.

Where practical, it should transport encrypted structured events rather than become the authoritative project database.

The server also hosts NYLRA's conversation and explanation logic. Relevant
topic context may be retained to continue the same discussion from a phone.
Fresh local retrieval and new Planner work require the daemon to be connected.

---

## 22. Security Model

Security must be designed from the beginning.

### Core principles

- least privilege,
- explicit scopes,
- device trust,
- revocable credentials,
- short-lived tokens,
- encrypted transport,
- encryption at rest,
- strong auditability,
- human confirmation for sensitive actions.

### The assistant should never become a hidden superuser

Every capability should be scoped.

Example scopes:

```text
project.read
plan.read
decision.read
blocker.read

planner.message
planner.assessment.request

decision.submit
execution.pause.request
execution.resume.request
```

No wildcard permission should be required for normal use.

---

## 23. Approval Integrity

A sensitive approval should carry proof of origin.

A decision record may include:

```text
decision_id
user_id
device_id
session_id
channel
proposal_hash
timestamp
confirmation_method
signature
```

This makes it possible to answer:

- Who approved this?
- On which device?
- Through which interaction?
- What exact proposal did they approve?
- Did the proposal change afterward?

Approval should bind to the exact proposal content.

A model-produced `human` field or an automatically signed interpretation is
not proof of human confirmation. NYLRA's model cannot call an unrestricted
approval operation. The trusted confirmation flow and owner-side revalidation
are prerequisites in
[shared architecture sections 8 and 13](../../../shadows-backend/docs/architecture.md#8-decision-and-approval-integrity).

---

## 24. Auditability

The platform should have an immutable or append-only audit history for significant events.

Examples:

- planner raised blocker,
- Liaison retrieved context,
- Liaison explained issue,
- human proposed direction,
- planner evaluated direction,
- human approved,
- decision submitted,
- Shadows recorded decision,
- plan resumed.

This allows a complete postmortem without relying on chat transcripts.

---

## 25. Explainability and Provenance

Every important summary should be traceable.

When the Liaison says:

> "This will affect two projects."

The user should be able to ask:

> "Why?"

The assistant should be able to show:

```text
Based on:
- Plan P-14
- Plan P-31
- Contract A-8
- Decision D-71
- Dependency Map revision 42
```

Summaries should have provenance.

The system should distinguish:

- verified fact,
- planner opinion,
- Liaison inference,
- human preference,
- prediction,
- uncertainty.

---

## 26. Trust and Confidence

The Liaison should not pretend certainty.

Useful labels may include:

```text
Verified
Likely
Uncertain
Conflicting evidence
Planner opinion
Human decision required
```

If the system cannot resolve the context safely, it should say so and request clarification.

Trust should be a first-class UX property.

---

## 27. Multiple Models for Different Responsibilities

A future optimization can route tasks by difficulty.

Example:

```text
Event classification
    -> small local model

Simple summary
    -> low-cost model

Project explanation
    -> mid-tier model

Complex cross-project reasoning
    -> strong model

Voice transcription
    -> specialized STT
```

This keeps cost under control.

The strongest model should be used only when necessary.

---

## 28. Cloud Hosting and Future Deployment Options

NYLRA's selected operating model is cloud-hosted in `shadows-backend`, with
desktop and mobile clients reaching the same assistant conversation. This
replaces the earlier idea of a local assistant engine plus a relay-only server.

Shadows remains independently usable locally. Closing NYLRA or losing cloud
connectivity does not remove the local Shadows dashboard and runtime.
NYLRA can explain retained topic evidence while the source machine is offline,
with freshness visible. It cannot perform a fresh local read or run Planner
on a disconnected machine.

Self-hosting remains a future deployment option for privacy-sensitive users,
enterprises, and controlled networks. A fully local NYLRA reasoning engine is
a future option requiring its own approved design, not part of the selected
implementation or a promised language migration.

---

## 29. Privacy Architecture

Personal memory may contain sensitive preferences and conversations.

Project state may contain proprietary source code and designs.

These should be treated separately.

Cloud reasoning requires permitted plaintext to be processed by NYLRA and the
chosen provider. Do not promise end-to-end secrecy from those processors. Show
data movement and recipient providers. The
[sharing/lifecycle owner](../../../shadows-backend/docs/architecture.md#what-lives-where)
defines selection, derived-context scope and revocation. Retention, redaction and
deletion include cached summaries. Future local reasoning and encryption designs
need their own reviewed boundaries.

---

## 30. BYOK First, Managed Later

The first business model should prioritize BYOK.

Benefits:

- lower infrastructure cost,
- user choice,
- provider independence,
- easier experimentation,
- less model lock-in.

Later, the product may offer a managed plan:

```text
BYOK
- user supplies provider keys
- user pays providers directly

Managed
- assistant includes model usage
- unified billing
- managed voice
- managed reliability
```

The architecture should support both without redesigning the core.

---

## 31. Repository Ownership

```text
shadows/
    Rust project rules, local runtime and future team server/assigned workers

shadows-backend/
    Shared cloud application
    NYLRA cloud conversation, context, and personal memory
    Accounts, device trust, selected-project links, and reliable delivery
    Team identity/membership and deployment composition
    Calls Rust project rules without owning their tables

nylra/
    Desktop client and character experience
    Flutter mobile client
    Client-side presentation and native platform integration
```

These are agreed repository responsibilities, not existing source folders or
a scaffolding plan. The [shared architecture](../../../shadows-backend/docs/architecture.md)
owns the technical boundaries and language decisions. Code is organized by
responsibility with source contracts following the
[supplied template](../../../shadows-backend/docs/contracts/TEMPLATE.yaml).

---

## 32. Protocol as a Product Boundary

The Shadows-to-Liaison protocol should be versioned independently.

Illustrative event, not an implemented Shadows event or a finalized wire schema:

```json
{
  "type": "planner.blocked",
  "version": 1,
  "project_id": "project-123",
  "plan_id": "plan-88",
  "blocker_id": "B-218",
  "severity": "high",
  "summary": "Persistence decision conflicts with scheduler requirement",
  "references": [
    "decision:D-71",
    "requirement:SR-12"
  ],
  "requires_human": true
}
```

The Liaison should not need the full Planner transcript to understand the event.

---

## 33. Human Intent as Structured Data

Natural conversation is convenient, but the final instruction should become structured.

Human:

> "Keep SQLite. Put an abstraction around it. Ask Claude first if that will hurt us later."

Structured intent:

```json
{
  "intent": "propose_direction",
  "constraints": [
    "preserve_sqlite",
    "introduce_storage_abstraction"
  ],
  "follow_up": {
    "type": "request_planner_assessment",
    "question": "Does this create material future risk?"
  },
  "approval_state": "not_approved"
}
```

The transcript is not the control plane.

Structured intent is.

---

## 34. Scheduled Briefings

The Liaison should eventually provide routine summaries.

Examples:

> Morning:
> "Three agents worked overnight. Two plans advanced. One decision is waiting for you."

> Evening:
> "Today, Project A completed 7 tasks. Project B is blocked on one API contract. No critical incidents."

Briefings should be customizable by:

- time,
- project,
- severity,
- format,
- text vs voice,
- maximum duration.

---

## 35. Smart Notification Batching

Ten small events should not produce ten interruptions.

The Liaison should batch related events:

> "Three related issues appeared in the authentication migration. They all come from one contract change. I can explain them together."

This reduces notification fatigue.

---

## 36. Critical Voice Calls

Phone calls should be rare and meaningful.

A critical call might be triggered when:

- production is down,
- a deadline is about to be missed,
- execution is blocked for too long,
- a security decision is required,
- a high-cost action needs approval,
- multiple projects are affected,
- an explicit user policy requires escalation.

The first sentence of a call should immediately answer:

1. what happened,
2. how urgent it is,
3. whether a decision is needed.

Example:

> "A production deployment is paused because the migration could delete user data. Nothing has been executed. I need your decision before the system continues."

---

## 37. The Liaison as a Personal Representative

Over time, the Liaison may learn how the owner prefers to operate.

Not by silently taking control, but by learning communication patterns.

Examples:

- prefers concise explanations,
- asks for alternatives before architecture changes,
- prioritizes reliability over speed,
- wants budget-impacting decisions escalated,
- accepts small refactors automatically under a certain risk threshold,
- wants all production actions confirmed.

This creates a personalized operating layer.

Automatic acceptance thresholds require explicit owner-enforced delegated policies.
Learning a preference does not grant authority.

---

## 38. Delegated Policies

A future capability may allow limited delegation.

Example:

> "You may approve dependency upgrades under $0 cost and low risk without calling me."

Or:

> "The engineering lead may approve UI changes, but architecture changes still require me."

Delegation should be explicit, scoped, reversible, and visible.

The system must never infer broad approval authority from casual conversation.

Attribute delegated outcomes to the policy and performing agent, not a fresh human
confirmation. Shadows §24.6 owns that distinction.

---

## 39. Multi-User and Team Mode

Although the first product may be personal, the architecture should not prevent teams.

Future roles:

- owner,
- engineering lead,
- product lead,
- reviewer,
- security approver,
- finance approver.

A decision could require different approval policies:

```text
Architecture
    -> Engineering Lead

Security
    -> Security Reviewer

Budget > threshold
    -> Owner

Production destructive action
    -> Two-person approval
```

The Liaison may then route the question to the correct person.

---

## 40. From Personal Assistant to Organizational Interface

The long-term opportunity is larger than one assistant.

An agentic company may contain:

```text
Human Leadership
      |
Human Liaison Layer
      |
Executive Agents
      |
Team Managers
      |
Planners / Critics / Researchers
      |
Executors
      |
Projects
```

The Liaison layer becomes the human interface to the synthetic organization.

Humans communicate with the system at the decision level rather than the transcript level.

---

## 41. Integration Beyond Shadows

The product should initially integrate deeply with Shadows.

But the protocol and architecture should be generic enough to support other systems later.

Possible future adapters:

- coding agents,
- CI systems,
- issue trackers,
- project management tools,
- workflow engines,
- research agents,
- autonomous operations systems,
- business automation platforms.

The core abstraction is:

```text
External Agent System
      |
      v
Adapter
      |
      v
Liaison Event Model
      |
      v
Human
```

This turns the product into a general **Human Liaison Platform**.

---

## 42. Replay and Decision Simulation

A powerful future feature would allow the user to replay or simulate a decision before approving it.

Example:

> "What happens if I choose option A?"

The Liaison could ask the Planner to simulate downstream effects without modifying the real plan.

Results:

```text
Option A
- 2 plans require revision
- estimated +3 tasks
- no contract breakage

Option B
- 1 future refactor
- no current plan revision
- medium technical debt
```

This makes approvals much more informed.

---

## 43. Counterfactual Explanations

The assistant should eventually answer:

- "What if we do nothing?"
- "What changed since yesterday?"
- "Why is this suddenly a problem?"
- "What would Claude do if I were not here?"
- "What is the safest option?"
- "What is the fastest option?"
- "Which option creates the least future work?"

This turns decision support into a core capability.

---

## 44. Decision Quality Feedback

After a decision produces real results, the system can compare expectations with outcomes.

Example:

```text
Decision D-184 expected:
- no current blockers
- small future scheduler refactor

Observed after 30 days:
- no blocked plans
- scheduler refactor required 2 tasks
```

This creates an evidence loop.

The human and Planner can improve decision quality over time.

---

## 45. Attention Budget

Human attention should be treated as a scarce resource.

The Liaison could maintain an "attention budget."

Instead of asking five small questions independently, it may combine them:

> "I have three decisions for you. Two are low risk and one affects architecture. We can handle all three in about four minutes."

This is especially important when agent teams scale.

---

## 46. Cost and Budget Awareness

### Production companion

NYLRA should proactively explain operational incidents from linked AWS environments:
"The orders service has sustained pressure and increased latency. The current
evidence points to application capacity; here is the proposed change, its cost
assumptions and the approval needed." This is an illustrative interaction, not a
measured diagnosis. High CPU alone must not become a confident scaling request.

The conversation continues after the action: did latency recover, did errors fall,
was the change ineffective, or is more evidence required? Scaling, reducing unused
capacity, investigating a slow query and identifying a bad release are possible
outcomes. NYLRA reports unknown/stale evidence and pending actions honestly.

[Shadows §17.12](../superpowers/specs/2026-10-01-task-contracts-and-evidence-design.md#1712-production-operations-extension)
owns observation-to-action authority and verification. NYLRA owns explanation,
attention and the human interaction. The feature remains part of the product
regardless of competition entry. Voice and AWS are integrations, not new owners
of project rules or implied permission to alter infrastructure.

The Liaison can eventually understand financial context.

Examples:

- model spend,
- cloud spend,
- expected token cost,
- API usage,
- infrastructure cost,
- deployment risk.

A planner may say:

> "Option A is technically better."

The Liaison may add:

> "It also increases expected monthly infrastructure cost by approximately 40%."

This gives the human a broader decision view.

---

## 47. Travel and Offline Mode

The mobile app should eventually support partial offline behavior.

Possible offline capabilities:

- read cached briefings,
- inspect recent decisions,
- draft a response,
- record a voice response,
- queue an approval for later secure submission.

High-risk approvals may require an online verification step.

Every queued confirmation retains its exact proposal basis and is validated
by the owning runtime on delivery. A superseded or changed proposal cannot
be accepted merely because its earlier response was queued offline.

---

## 48. Wearables

Future wearable experiences could be extremely useful.

Examples:

- watch notification:
  > "Planner needs architecture approval."

- quick actions:
  - Approve
  - Reject
  - Explain on Phone

The watch should not become a full project dashboard.

It should remain an attention and escalation surface.

---

## 49. Presence Without Distraction

The assistant should feel present but not noisy.

The ideal personality is:

- calm,
- competent,
- concise by default,
- detailed on request,
- transparent about uncertainty,
- respectful of user attention,
- never pretending to have authority it does not have.

It should feel like a trusted technical chief of staff, not a cartoon notification generator.

---

## 50. Personality and Identity

The assistant may have:

- a name,
- a voice,
- an avatar,
- subtle animations,
- emotional tone,
- status reactions.

But personality should never obscure system state.

If a critical deployment is blocked, the assistant should be clear and professional.

The personality layer should be optional.

Enterprise users may prefer a minimal professional interface.

Individual users may prefer a warm companion.

---

## 51. Model Failure Boundaries

The assistant will occasionally misunderstand.

The architecture should expect that.

Safeguards:

- never infer approval from ambiguous speech,
- confirm high-impact decisions,
- bind approvals to exact proposal content,
- show uncertainty,
- allow "show me the source",
- separate assistant interpretation from project facts,
- keep strong audit history,
- allow easy correction.

The goal is not perfect AI.

The goal is safe coordination around imperfect AI.

---

## 52. Non-Goals

The first versions should **not** try to become:

- a full coding IDE,
- a replacement for Shadows,
- an unrestricted computer-use agent,
- a general social chatbot,
- a direct database editor,
- a silent autonomous manager,
- a universal voice assistant for every consumer task.

The product is specifically about human supervision of agentic work.

---

## 53. Full Capability Inventory and Delivery Boundaries

The full product direction is preserved. This inventory is not an approved
implementation sequence or a claim that any capability is implemented.
Delivery uses complete runnable paths inside the selected architecture;
it does not plan a rewrite when the product grows.
The [delivery order](../../../shadows-backend/docs/architecture.md#14-delivery-order)
starts with sourced text continuity, enforced assessment/confirmation, then voice
and presence. Team work and cloud execution retain separate runtime gates.

### Foundation and trust

- Shared cloud-hosted NYLRA conversation and personal memory.
- Account/device trust and optional linking of selected Shadows projects.
- Bounded current-work context, source provenance, and visible freshness.
- Planner assessment requests, explicit confirmation, owner-side validation,
  durable delivery, and audit evidence.
- An explanation model separate from the planning model.

### Desktop and mobile continuity

- Desktop companion, optional AYLUL character, tray presence, and local navigation.
- Flutter mobile client, notifications, and remote topic responses.
- The same NYLRA topic continued across devices.
- A local connector that survives closing the desktop interface.
- Explicit disconnected-source behavior and recoverable pending responses.

### Voice and attention

- Independently selected STT, reasoning, and TTS providers.
- Conversational voice, text fallback, and voice shortcuts.
- Quiet hours, severity policies, grouped notifications, and scheduled briefings.
- Future telephone-call escalation with its own channel rules.

### Advanced personal assistance

- Richer personal memory with correction, retention, and forgetting controls.
- Multiple linked instances and isolated project contexts.
- Cost/token visibility, attention budgets, decision replay, and outcome feedback.
- Source-bound explanations and clear distinctions between fact and inference.

### Extended platform

- Non-Shadows integrations and an adapter SDK when another real consumer exists.
- Self-hosting, enterprise identity, and team approval workflows.
- Future cloud Shadows workspaces and execution handoff under their own owners.
- Future production and infrastructure operations under explicit Shadows authority.

---

## 54. Responsibility Map

These responsibilities live within the shared backend and its client adapters
as appropriate. The exact ownership boundaries are defined in
[shared architecture section 3](../../../shadows-backend/docs/architecture.md#3-application-organization).
This conceptual map does not require a separate service or package per branch.

```text
Liaison Core
|
+-- Identity
|   +-- Accounts
|   +-- Devices
|   +-- Sessions
|
+-- Pairing
|   +-- Shadows instances
|   +-- Scoped credentials
|
+-- Protocol
|   +-- Events
|   +-- Commands
|   +-- Versioning
|
+-- Context Engine
|   +-- Retrieval
|   +-- Compression
|   +-- Provenance
|
+-- Memory
|   +-- Personal preferences
|   +-- Conversation memory
|   +-- Retention policy
|
+-- Providers
|   +-- LLM
|   +-- STT
|   +-- TTS
|
+-- Policy Engine
|   +-- Severity
|   +-- Escalation
|   +-- Quiet hours
|
+-- Decision Gateway
|   +-- Proposals
|   +-- Human confirmation
|   +-- Signatures
|
+-- Channels
|   +-- Mobile
|   +-- Desktop
|   +-- Push
|   +-- Voice
|   +-- Calls
|
+-- Audit
    +-- Event history
    +-- Decision proof
```

---

## 55. Example End-to-End Scenario

Claude Planner raises:

```text
BLOCKER B-218
Severity: high

The current persistence decision conflicts with the shared scheduler design.

References:
- Decision D-71
- Scheduler Requirement SR-12

Human decision required.
```

The Liaison retrieves both references.

It tells the user:

> Yesterday you approved SQLite for local workspace storage. The new scheduler design introduces coordination across workspaces, so Claude is unsure whether to change the storage architecture now or keep SQLite and add an abstraction. No code has been changed.

The user asks:

> "Didn't we agree not to replace the database?"

The Liaison responds:

> Yes. Decision D-71 explicitly says to keep SQLite during this stage. The new issue is that this decision was made before SR-12 introduced shared scheduling.

The user says:

> "Keep SQLite. Put an abstraction around it. Ask Claude whether that will cause problems later."

The Liaison creates:

```text
Proposal P-184

Human direction:
- Preserve SQLite.
- Add storage abstraction.
- Revisit distributed persistence later.

Request:
Planner assessment only.

Approval:
Not granted.
```

Claude answers:

```text
Assessment A-184

Viable: yes
Current plan breakage: none
Future cost: small scheduler refactor
Recommendation: proceed
```

The Liaison tells the user:

> Claude agrees. It expects a small scheduler refactor later, but this will not block any currently approved plan. Approve?

The user says:

> "Yes."

The person confirms the exact proposal in the trusted client surface.
The Decision Gateway then records that explicit confirmation.

The shared backend delivers the confirmed response to the owning Shadows runtime.

Shadows revalidates authority and the proposal basis, then records acceptance.
Planner applies the accepted direction through the project's service rules.
NYLRA reports the confirmed outcome; resumed work is reported only when observed.

The user never opened the IDE.

The repeated explanation dialogue used NYLRA rather than additional Planner
turns. Actual cost savings require measurement across the complete journey.

The system remained auditable.

The storage-abstraction examples illustrate a dialogue, not a recommendation to
add a layer to Shadows. Actual proposals must satisfy its library-first and
first-real-user rules.

That is the product.

---

## 56. Why This Matters

As agent systems become more capable, the bottleneck moves.

First, the bottleneck was code generation.

Then it became planning.

Then orchestration.

Eventually, the bottleneck becomes **human attention and decision communication**.

A person cannot efficiently supervise dozens of agents by reading their transcripts.

They need a trusted interface that understands both sides:

- the technical system,
- and the human owner.

This is the gap the Liaison is designed to fill.

---

## 57. Long-Term Vision

The long-term product is not merely a Shadows companion.

It is a new interface category.

Today:

```text
Human
  |
Dashboard
  |
Agents
```

Future:

```text
Human
  |
Trusted AI Liaison
  |
Synthetic Organization
```

The Liaison knows:

- what the organization is doing,
- where it is blocked,
- which decisions matter,
- what the human previously decided,
- how the human prefers to work,
- and when to interrupt.

The human should be able to manage an entire AI-assisted organization through natural conversation without surrendering control.

The system becomes increasingly autonomous.

The human becomes increasingly strategic.

The Liaison protects the boundary between the two.

---

## 58. Product Principles

1. **Human authority remains explicit.**
2. **The assistant explains; it does not impersonate the human.**
3. **Planner intelligence should be spent on planning, not repetitive explanation.**
4. **Project truth and personal memory must remain separate.**
5. **Every important claim should be traceable to evidence.**
6. **Every important approval should be auditable.**
7. **Voice is a channel, not a model requirement.**
8. **The user chooses providers whenever possible.**
9. **Shadows remains useful without the assistant.**
10. **The assistant must not depend on Shadows' database internals.**
11. **Human attention is a scarce resource.**
12. **Interrupt only when the value justifies it.**
13. **Security boundaries are product features, not implementation details.**
14. **Uncertainty should be visible.**
15. **Voice degrades to text; disconnected sources expose freshness and limits; Shadows remains independently local-first.**
16. **One Liaison should eventually be able to represent many projects, agents, and systems.**
17. **Personality is optional; trust is mandatory.**

---

## 59. Manifesto

We do not want humans to babysit agents.

We do not want planners wasting context explaining the same history repeatedly.

We do not want critical decisions buried in dashboards.

We do not want assistants silently making decisions they were never authorized to make.

We want a system where agents can work deeply, humans can stay informed, and authority remains clear.

The Planner should plan.

The Executor should execute.

The Critic should challenge.

The Researcher should investigate.

The Executive layer should coordinate.

The Liaison should understand all of them and speak to the human.

The human should be able to say:

> "Explain what happened."

Then:

> "Ask the planner what it thinks."

Then:

> "Okay. Approve that."

And the entire agent organization should understand exactly what that means.

That is the vision.
