# Agent workspace UI batch (U-W1 to U-W3)

Recorded 2026-10-19. Evidence, not design authority.

## Gate (from `apps/desktop/`, one run, exit 0)

`npm run build && npm test && npm run test:companion:unit && npm run test:companion:browser && npm run test:companion:production`

- build (tsc + vite): passed.
- `npm test`: 7 tests, 7 passed.
- `test:companion:unit`: 15 tests, 15 passed.
- `test:companion:browser` (Playwright): 61 passed in 1.7 minutes.
- `test:companion:production`: 1 test, 1 passed.

No protocol change, so no Core Cargo gate.

## Native build

`cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml` failed in
the default target dir: `failed to remove file ...\target\debug\nylra-desktop.exe,
Access is denied`, because a `nylra-desktop.exe` process (pid 10712) was running and
locked it. That process was not stopped. The same command with
`CARGO_TARGET_DIR` set to a temporary folder compiled and finished (4m 34s). The
default debug exe is therefore the previous build until the app is closed and
rebuilt.

## Hands-on journey (not yet done)

To be run by Mohammed on the rebuilt debug app, then recorded here: sign in; chat
and leave a draft, navigate away and back, the draft remains; approve one request
under Activity; open Settings and see account actions; hide and show the
character; read the app logs afterwards. Browser fixtures do not count.

Result: PENDING.

## Remaining gates

Backend/character gates are unchanged from the
[normal-chat evidence](2026-10-19-normal-chat.md). Whole-branch review: pending.
