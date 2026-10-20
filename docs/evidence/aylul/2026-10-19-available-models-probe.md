# ACP available-models prerequisite

Recorded 2026-10-19, Asia/Aden, on branch `feat/aylul-profiles`, baseline `3e89dab`.
Real installed `claude-agent-acp` 0.81.1, Node 24, isolated temporary home,
synthetic authentication token and loopback port 9. No prompt or provider request
was sent. Client capabilities matched `shadows-agent/src/acp.rs` including the
`jetbrains.air` `recommendedValue` capability. Session setup used only the `user`
settings source.

User settings:

```json
{"availableModels":["MiniMax-M3","MiniMax-M2.7"],"model":"MiniMax-M3"}
```

The real `session/new` response reported model values `default`, `MiniMax-M3`,
`MiniMax-M2.7`; current model `MiniMax-M3`. Prerequisite PASS: both configured ids
were present and the current model was one of them. `default` must be removed
from AYLUL choices by Shadows. Both custom choices had the display name
`MiniMax-M3`, despite distinct ids; Shadows must use the configured ids as labels.

This proves adapter configuration discovery only. It does not prove provider
authentication, real answers, model switching, or browser acceptance.
The temporary probe client was not added to the product source.
