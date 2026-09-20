# ADR-0001 — Foundation and ownership

- **Status:** accepted
- **Date:** 2026-09-20
- **Detail:** canonical design specification

## Decisions

- Build Shadows as a modular monolith: one Rust crate, one daemon/CLI binary,
  and an independent browser client.
- Prefer mature libraries where they match the required semantics. Add a seam
  only for known volatility or a boundary that must be protected.
- Shadows owns durable project truth. Claude, Codex, native sessions, browser
  connections, and other clients are replaceable workers or caches.
- Role, harness, provider, and model are separate concepts.
- Modules own their infrastructure: `storage/` owns SQLx, `protocol/` owns
  HTTP/SSE, `process/` owns child processes, and `secrets/` owns secret
  resolution.
- Use module privacy first and small architecture checks only as mechanical
  defense in depth.

## Delivery constraint

Only modules needed by the current vertical slice are implemented. A module
listed in the complete design is not automatically part of the first release.

