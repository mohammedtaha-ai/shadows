---
name: feedback-open-value-sets
description: Mohammed's ruling 2026-10-21 - a list the clients own (colours, etc.) is stored as an opaque id with a shape check, never a backend enum
metadata:
  type: feedback
---

When a value's list belongs to the clients (the character colours were the case), Core stores
an opaque id with only a shape check and a default; the clients draw a fallback for an id they
do not know. No backend enum.

**Why:** Mohammed (2026-10-21) wants adding a value later to need no backend change. He first
gave six colours, then removed red, then ruled the enum out entirely.

**How to apply:** for any new field whose values the frontend or renderer owns (themes beyond a
fixed trio, languages, icons), propose an opaque id plus a shape check and say who owns the list
in the contract's agreements. Fixed enums only where Core itself branches on the value.
