I attached `shadows-web-sync.zip`. It holds our latest frontend work, made in the real Shadows repository after the version you started from. Bring this app up to date with it.

The goal is to make merging this app back into our repository easy. Our repository's `web/src/` maps 1:1 to this app's `src/`.

## What the zip contains
- `src/...`: the complete current versions of 25 files. Each path matches this app's `src/` path.
- `changes.patch`: a unified diff from your starting version to those files, so you can see exactly what changed.

The changes are:
1. **MCP tools in the extension library.**
   - New form: `src/app/settings/mcp-form.tsx`.
   - Wired into `extension-list.tsx` and `sections/plugins.tsx`.
   - API in `src/api/extensions.ts`: `McpItem`, `McpInput`, `McpValueInput`, `saveMcp`.
2. **Agent capabilities.** The agent form's extension switches now cover skills, plugins **and MCP tools**: `agent-extensions.tsx` and `agent-settings.tsx`.
3. **Permission card.**
   - When an agent asks before running a tool, the conversation shows a card with the offered option buttons: `src/app/conversation/permission-card.tsx`, rendered from `entry.tsx`.
   - API: `src/api/permissions.ts`.
4. **Live refresh.** A `PermissionsChanged` project event refreshes the waiting permissions: `src/stream/refresh.ts`.
5. Small fixes in `outfit-picker.tsx`, plus tests and `contract.yaml` files.

## Rules
1. **Keep your design.** Restyle the new pieces (MCP form, capability switches, permission card) in the same visual language you already built: colours, glow, spacing, icons, Tailwind classes.
2. **Keep behaviour and logic exactly as in the zip.** Do not change:
   - API calls, request and response shapes, or query keys;
   - command ids, validation, or which buttons send what;
   - accessibility labels or text the tests look for.

   Change markup and classes only.
3. **Keep every feature I asked you to add earlier,** even ones the real backend does not support yet, such as task search and the agent workspace cards. Do not remove them. We will build their backend later.
4. **Keep the file layout.** Put each file at the same path as in the zip.
   - Do not rename, merge or split files.
   - Do not move logic into new files.
   - For a feature of your own that needs a new file, keep it in a new file of its own. Never mix it into one of the zip's files.
5. **Keep the tests and the `contract.yaml` files from the zip.**
   - Update a test only where your restyle changed a class or structure it checks.
   - Never change what a test asserts about behaviour.
6. **Mock backend in `server.ts`.** Add mock routes so the new screens work in this preview:
   - MCP library:
     - `GET /api/aylul/extensions` returns `{ plugins, skills, mcp }`.
     - An MCP item is `{ id, name, description, transport }`. The transport is either `{ type: "http", url, headers: [{ name, set }] }` or `{ type: "stdio", command, args, env: [{ name, set }] }`.
     - `POST /api/aylul/extensions/mcp` and `PUT /api/aylul/extensions/mcp/:id` take `{ command_id, name, description, transport, values: [{ name, value }] }`.
     - Never return secret values, only `set: true/false`.
   - Agent switches: `GET /api/aylul/agents/:slug/extensions` returns `{ plugins, skills, mcp }`, and each item has `enabled`.
   - Permissions:
     - `GET /api/aylul/permissions` lists the waiting requests.
     - `POST /api/threads/:id/session/permissions/:request_id` with `{ command_id, option_id }` returns `{ thread_id, request_id, option_id, kind, answered_at }`.
     - Errors: 404 `PERMISSION_NOT_FOUND`, 409 `PERMISSION_SETTLED`, 422 `PERMISSION_OPTION_NOT_OFFERED`.
     - A waiting request is a thread entry of kind `PermissionAsked`. Its card holds `request_id` and `options: [{ option_id, name, kind }]`, where `kind` is `allow_once`, `allow_always`, `reject_once` or `reject_always`.
     - Answering adds a `PermissionAnswered` entry and emits a `PermissionsChanged` event `{ request_id, state }` on `/api/projects/:project_id/events`.
   - Add one demo thread that shows a waiting permission card.
7. **Do not add npm packages** beyond what `package.json` already has.
8. Fix the import in `src/app/agents/character-colors.ts` to `'../../../assets/aylul/character-colors.json'`. The path has one `../` too many and fails when the app runs outside AI Studio.

When you finish, list:
- every file you changed;
- every file you added, with what each new feature is;
- any place where you could not keep a zip file's behaviour exactly, and why.
