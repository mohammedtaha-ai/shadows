# Shadows Web client

The first client of the Shadows daemon (spec §1: the daemon does not serve or
embed the client). React + TypeScript on Vite, TanStack Router and Query,
shadcn/ui (Base UI) on Tailwind v4, Motion for transitions, and Streamdown
(with its Shiki code plugin) for the Planner's replies.

## Run it

Two terminals. The daemon, from the repository root:

```sh
cargo run --release -- serve --harness "C:/Users/<you>/.local/bin/claude.exe"
```

It listens on `http://127.0.0.1:4318` and, unless told otherwise, accepts
requests from Vite's dev server (`http://localhost:5173` and
`http://127.0.0.1:5173`). Serving the client from anywhere else needs
`--allow-origin <origin>` on the daemon.

The client, from `web/`:

```sh
npm install
npm run dev
```

Then open `http://localhost:5173` in any browser. To reach a daemon somewhere
other than `http://127.0.0.1:4318`, set `VITE_SHADOWS_URL` (for example in
`web/.env.local`).

## The API types

`src/api/schema.d.ts` is generated from the daemon's OpenAPI document,
`../api/openapi.json`, and committed. After a protocol change (the daemon side
is `UPDATE_OPENAPI=1 cargo test --test openapi`), regenerate it:

```sh
npm run gen:api
```

CI regenerates it and fails on any difference. Every HTTP call goes through
`src/api/client.ts`; the event stream is read by the hand-written
`src/stream/` hook, because its replay-then-live contract (spec §2.10) is not
something OpenAPI can describe.

## Checks

```sh
npm run gen:api && git diff --exit-code -- src/api
npm run typecheck
npm run lint
npm test
npm run build
```
