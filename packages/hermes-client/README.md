# Native Hermes client

TypeScript transport extracted without behavioral changes from
`shadows-backend` commit `4dc5280`, `src/integrations/hermes/{client,target}.ts`.
The original focused HTTP fixture tests are retained here. Account authority,
conversation persistence, process supervision and credentials resolution belong
to callers. Runtime dependencies: Node 24 only.

Run `npm ci`, `npm test`, then `npm pack --pack-destination <vendor-directory>`.
The artifact contains compiled ESM, declarations and source. Consumers install
the archive with lockfile integrity; no sibling checkout is required at runtime.
