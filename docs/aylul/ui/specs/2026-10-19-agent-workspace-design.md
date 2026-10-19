# Agent workspace: bots, conversations and selected-bot presence

Date: 2026-10-19. Status: proposed design for human review; no product implementation
is authorized by this document alone. The human requested the experience below
and selected one visible desktop character for the selected bot.

## 1. Intended result

The desktop manages multiple persistent assistant bots. The person can create
AYLUL as a work-observation assistant and create other bots with different roles,
instructions, model/provider choices and appearances. Chat, bot administration
and settings precede the later project-observation/code-context integration.

The screenshots of installed Hermes Desktop are the functional reference for
bot creation, conversation navigation and configuration. Our existing React/Tauri
shell, account backend, native credential boundary and character renderer remain
the product owners. Existing account conversation history and hook approvals stay
usable throughout the transition.

Confirmed presentation choice: each bot can have its own character configuration,
but **one desktop character window displays the selected bot**. Background bots
retain their own activity and unread/attention state without taking over that
character or stealing focus.

## 2. Inspected baseline and approach

- The current assistant feature owns account conversations through native
  `assistant_call` and backend contract 0.5.0. It has no bot registry or per-bot
  runtime binding.
- Backend `assistant` currently targets one configured Hermes client. Its source
  contract explicitly excludes multiple independent memory profiles.
- Installed Hermes Desktop implements profile administration in
  `apps/desktop/src/api/profiles.ts` and Bot Mode through
  `plugins/hermes-bots/{profile-config,model-picker,profile-ops}.tsx/.ts`.
  The advanced editor consumes `profiles.describe`, `profiles.configure`,
  `model.options` and MCP catalog operations.
- Hermes Desktop's owner guide identifies its `tui_gateway` JSON-RPC connection.
  The native REST server selected for our chat is a different surface; its static
  capability flags include `admin_config_rw:false`, `jobs_admin:false` and
  `audio_api:false`. A profile-management route cannot be assumed to exist there.
- Our companion catalog already has idle, thinking, working, listening, speaking,
  success, blocked and sleeping expressions. `DesktopCompanion` currently supplies
  idle presentation; it does not yet receive actual bot activity.

Recommended approach: build our workspace over explicit account-backend
capabilities, reusing Hermes's runtime behavior through a proved management
adapter. This preserves native Windows/character ownership and remote placement.

Alternatives considered: adapting Hermes Desktop wholesale offers broader UI
coverage sooner but couples our shell/lifetime to its Electron/plugin stack;
embedding its complete dashboard introduces another navigation/auth/host bridge.
Neither is selected for this design. No replacement agent loop is proposed.

## 3. Main layout

```text
Navigation / lists         Conversation                         Inspector (optional)
-----------------------    ----------------------------------   --------------------
Chats | Bots               Bot icon, name, role, actual status   Bot details
Search                     Message / activity history           Instructions
New chat / Create bot      Tool progress when supplied          Model/provider
Pinned items               Source-linked attention cards        Capabilities
Account/settings footer    Composer + model selection           Character preview
```

The workspace fills the native window. Only the conversation history scrolls
within the chat area; the composer stays visible at its bottom. Lists and the
optional inspector scroll independently. Narrow windows collapse the inspector
and expose the list through a drawer. Arabic messages use automatic direction;
mixed English identifiers remain readable.

The account footer opens account/settings pages. Account/device UUIDs, device
enrollment, sign-out-all and revocation move to an explicit account/security view.
The main conversation retains a small truthful connection indicator and a useful
recovery action. A temporary backend outage preserves readable saved history.

## 4. Bots and conversations

Bot identity and conversation identity are separate. Every bot has an opaque
backend ID and a persistent runtime/profile binding; display names are editable
labels. Every conversation belongs to one bot. Selecting a bot opens its ongoing
primary conversation; additional chats can be created for that bot. The Chats
list provides history across bots with a visible bot badge and filter.

Create bot collects name, role/description, instructions, provider/model and an
available character choice. Advanced tabs cover memory policy, skills, toolsets,
MCP/integrations and monitoring connections when supported. Defaults and inherited
values are shown explicitly. Creating AYLUL with a monitoring role does not claim
Claude/Codex observation is active until its later binding exists.

The create flow reports draft, saving, ready or failure. A failed/unknown save
retains its command identity. It cannot leave a visually successful bot backed by
no runtime, or silently route it to another bot. Bot removal, history policy and
the effect on its profile/data require a specific preview/confirmation.

Existing 0.5.0 conversations must be preserved under a backend-selected legacy
bot binding during migration; no existing profile, account or provider history
is imported or deleted automatically. The migration's source schema and ownership
are part of the backend implementation plan, not an invented frontend mapping.

## 5. Configuration and the Hermes feature inventory

Global settings own account/security, language/theme, desktop behavior,
notifications and reusable provider connections. Bot settings own role,
instructions, default model/provider, memory/capabilities and character mapping.
Conversation settings own any explicit override and show the effective selection.
A bot's defaults do not silently retarget an already-running turn.

The composer shows the effective provider/model. Changes apply to a subsequent
turn; active or uncertain dispatch cannot be redirected by changing a selector.
Unsupported model options remain visibly unavailable rather than pretending to
have been saved. Provider secrets use the backend's established protected-write
pattern and metadata-only readback; stored values never return to the renderer.

The backend's existing provider-settings service is useful implemented work, but
its old provider-chat dispatch is not evidence that a setting configures Hermes.
The new bot path needs an explicit credential/runtime resolver contract. Selection
must retain account ownership, revision and runtime capability checks.

| Reference capability | Product destination | Delivery rule |
| --- | --- | --- |
| Chats, bots, search/pinning, creation | Workspace/list owners | First workspace slice |
| Bot rename, duplicate and profile import/export | Bot administration | Supported operations with explicit data/lifecycle effects |
| Provider/model and supported model options | Composer and bot settings | Real backend capability before enabled saves |
| Role/instructions, memory | Bot settings | Separate bot runtime/profile ownership |
| Skills, toolsets, MCP/connectors | Bot capabilities | Backend inventory and explicit allowed actions |
| Scheduled jobs | Jobs page | Separate job operations/lifecycle; not simulated timers |
| Messaging integrations | Integrations page | Connection-specific setup and capability checks |
| Artifacts and previews | Optional inspector/artifact view | Actual owned artifact references |
| Voice | Voice settings/composer controls | Requires separately implemented capture/playback |
| Appearance, notifications, shortcuts | App settings | Native/local presentation ownership |
| Advanced diagnostics and files | Explicit advanced view | Bounded authorized capability, no raw home exposure |

This inventory preserves the requested breadth; it is not a claim of complete
setting-by-setting parity. Before the implementation plan, inventory the installed
Hermes settings manifest and live configuration schema into supported, deferred
or platform-specific rows, including plugin-provided settings. A page whose operation is not
implemented must say so and disable its action; it must not submit to a guessed
Hermes route. Staging the delivery does not silently remove these capabilities.

## 6. Selected-bot character

Each bot stores a reference to an available character pack and its presentation
preferences. The existing AYLUL renderer/pack is the first available option;
new character artwork is its own character-domain task. This feature does not
claim distinct artwork already exists for every future bot.

Selection carries account, bot ID and a generation so late activity from an old
selection cannot change the current character. Hiding the main window preserves
the selected character under the existing native lifetime/render-permission
rules. Hiding/failing the character preserves chat and monitoring.

| Observed fact | Presentation |
| --- | --- |
| No active work | Idle or explicitly chosen resting appearance |
| Accepted active model turn | Thinking/working with the actual bot label |
| Active search operation reported by its owner | Searching; an explicit activity label even if a new animation is pending |
| Person interacting in the selected chat | Chat presence; not evidence that a task is running |
| Native microphone capture actually active | Listening |
| Audio playback actually active | Speaking |
| Actual refusal or attention item | Blocked/attention with its source |
| Completed owned operation | Brief completion indication, scoped to that operation |
| Lost connection/unknown outcome | Offline/unknown; never a fabricated success |

Background activity updates its bot's row/badge. Selecting its notification is
an explicit way to switch context. No automatic bot/character switching and no
new focus-stealing window. A styled character does not grant tool authority.

## 7. Runtime boundaries and delivery order

The desktop continues to call native typed operations against our single account
backend. Backend owns bot IDs, account checks, profile binding, command receipts
and public generated schema. Hermes owns its profile/model behavior. The local
sidecar keeps hook/session/approval ownership; Core and code intelligence remain
independent callers/services. No global replacement AppCore is introduced.

Delivery order:

1. Build a reviewable workspace presentation; explicit test data for missing bot
   operations lives only in the preview/test harness. Retain real current
   chat/account paths in the product and disable unsupported actions. No simulated
   save is represented as a product-connected feature.
2. Prove one installed-Hermes management path, accept the backend bot contract,
   then implement durable bot creation/configuration and bind conversations to it.
   The user-visible connected slice includes a real create -> model selection ->
   send -> reopen journey; a visual prototype alone does not complete it.
3. Connect the one selected-bot character to real activity; preserve native ACL,
   window lifetime and existing character tests.
4. Add observed Claude/Codex/Core work and its context/control through their
   existing integration plans. Add jobs, richer capabilities and voice through
   their own proved operations while preserving this navigation.

## 8. Decisions still needing proof

**OPEN - Hermes management transport.** Close before bot CRUD is enabled with a
pinned, isolated capability trial for create/describe/configure/delete, model
inventory, profile isolation and restart. The REST chat server is not presumed to
offer the desktop's JSON-RPC administration. An internal supported gateway or
standalone integration adapter must be selected from that evidence.

**OPEN - Credential/model resolution for a bot.** Close before a real configurable
bot turn with an accepted backend request proving the selected connection reaches
only the owned profile, values never return in projections and changes fence
in-flight/unknown requests. No implicit fallback to the old provider chat loop.

**OPEN - Character/activity mapping.** Close before character status ships with
actual event sources, selection-generation tests, offline/unknown precedence and
native presence acceptance. Native audio/search are not inferred from prose.

## 9. Acceptance and handoff

UI checks cover chat/list/inspector layout at narrow/desktop sizes, keyboard use,
Arabic/mixed text, account settings separation and composer visibility. Behavioral
checks cover account/bot/conversation switches, delayed results, selected model
display, effective settings and unknown writes without automatic resend.

Backend acceptance covers bot/profile isolation, command replay/conflict, settings
revision races, credential-free projections and fresh authority after awaits.
Character acceptance covers one selected owner, unchanged hide/failure isolation
and truthful activity. Actual configured-provider and physical native journeys
remain separate from fixtures, and recorded with their exact limits.

After human review of this written design, write a scoped implementation plan
with exact owners/files and select execution method. Backend and character
changes follow their accepted contracts; nothing here marks those features done.
