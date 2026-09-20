# Secrets use references, never stored values

**Doc ID:** 989
**Status:** active
**Tags:** credentials, isolation, secrets, security
**Source slug:** secrets-use-references-never-stored-values

---

# Decision — Secrets use references, never stored values

**Status:** active · 2026-09-20

## Rule

```text
Config           → SecretRef only (NEVER values)
SecretResolver   → resolves SecretRef → ephemeral value
Agent spawn      → injects into child env at spawn time only
```

`config/` and `secrets/` are separate concerns:

- `config/` holds non-secret configuration + `SecretRef`s.
- `secrets/` resolves `SecretRef`s at runtime. It is the ONLY place
  where secret values live in memory.

## Forbidden

- Storing credential values in config files.
- `std::env::set_var` for credentials.
- Credential inheritance from daemon to child.
- Credentials in SQLite, events (durable or transient), tracing,
  errors, serialized config, command-line args.
- `HOME` / `USERPROFILE` / `APPDATA` / temp / config inheritance
  between daemon and child without explicit policy.

## Required

- Every child process gets its own explicit environment.
- Credentials are injected only at spawn time, from a SecretResolver.
- The SecretResolver is the only place credentials exist in memory.
- Workers do not pollute each other's config / cache / home.

## Where secret values may appear

- In the child process environment, injected at spawn time only.

## Where secret values may NOT appear

- SQLite
- Event log (durable or transient)
- Tracing / logs
- Error payloads
- Serialized config files
- Process command-line arguments

## Implementation note

In v1, `secrets.rs` may live at the crate root or as a small
sub-module. The rule is the responsibility split: config has refs,
SecretResolver has resolution, agent spawn has injection. The
value's lifetime is spawn to child termination.

Related: [[parallel-execution-isolation]], [[process-owns-child-process-primitive]]
