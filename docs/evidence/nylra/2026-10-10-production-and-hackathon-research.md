# Production direction and optional hackathon fit

Date: 2026-10-10. Research only; no deployed integration, entry or eligibility
approval is claimed. Production is the user's independent product feature.
Its owner is [Shadows §17.12](../../../shadows/docs/superpowers/specs/2026-10-01-task-contracts-and-evidence-design.md#1712-production-operations-extension).
This record does not change that design or the full delivery roadmap.

## Sources and verification

The user supplied the full rules as `Pasted text.txt`, including the September 16
repository update and October 7 credit update. The official rules URL was readable
in this research run, unlike the earlier failed homepage fetch. The supplied text
is source material, not authorization to register, invite collaborators, publish
repositories or deploy resources. Summary below is principally from that supplied
text, cross-checked against the official page; recheck amendments before entry.

- [Official rules](https://amazonappdev2026.devpost.com/rules).
- [Organizer simulation clarification](https://amazonappdev2026.devpost.com/forum_topics/45058-clarification-on-simulated-alexa-web-experience-requirements).
- [Amazon event listing](https://developer.amazon.com/apps-and-games).
- [MCP Streamable HTTP](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports).
- [CloudWatch overview](https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/WhatIsCloudWatch.html).
- [CloudTrail overview](https://docs.aws.amazon.com/awscloudtrail/latest/userguide/cloudtrail-user-guide.html).
- [CDK deployment/change sets](https://docs.aws.amazon.com/cdk/v2/guide/ref-cli-cmd-deploy.html).

CloudWatch provides application/resource observability; CloudTrail records AWS
account/API activity. They are possible evidence sources, not a ready-made Shadows
diagnostic engine. CDK supports preparing a change set before execution. None of
these capabilities proves our authority binding, incident diagnosis or recovery.

## Competition facts relevant to a possible entry

- Submission closes October 23, 2026 at 12:00 Pacific daylight time: 22:00 Aden.
  Judging ends November 20 at 12:00 Pacific standard time: 23:00 Aden.
- AWS Builder is an additional challenge requiring a primary-track project.
  Provisioning RDS/S3 alone is not a primary-track entry.
- Alexa+ is the closest potential fit. Its actual integration route requires the
  specified working Agent Skill/MCP implementation; the rules specify MCP
  2025-11-25 or confirmed later, using Streamable HTTP. Existing Shadows MCP
  compatibility was not tested here and must not be assumed from its name.
- The alternate simulated Alexa+ route permits an AI/agentic-tool-built simulation
  without a required framework or MCP surface. Its source and functioning demo
  still belong in the submission. The organizer's linked clarification says voice
  and Alexa-like UI are not required; a real backend with simulated interaction is
  permitted. This does not certify an arbitrary chatbot as an eligible submission.
- A GitHub repository must provide all needed code, assets and setup/run guidance.
  A public submission needs an open-source license; a private one must grant the
  specified judging access. Recheck the current access list rather than copying
  names into automation. For our multiple repos, pin required revisions and make
  every dependency accessible with one reproducible entry guide.
- Provide an English description/documentation or translations, a publicly visible
  YouTube/Vimeo demonstration under three minutes, track selection and product
  feedback for the tools used. Existing projects must explain significant work
  within the contest window. Testing access must last through the judging period.
- AWS Builder needs documented integration and tool feedback. The rules also allow
  Kiro Crew development use to qualify without an additional runtime AWS service.
  The October 7 update says promotional credit codes are exhausted; do not budget
  on receiving the earlier advertised credits.
- Open Source is a separate additional contribution requirement, not fulfilled by
  these design-document edits alone. Record the required contribution/repository
  URLs, author and meaningful work if pursuing it.
- Judging considers implementation, design, impact and idea quality equally;
  optional actionable friction logs may add up to 10%. We should demonstrate a
  useful operational journey rather than add AWS services for their names.
- Individual/team eligibility, ownership, licenses and current rules still need
  entrant review. Timezone is not evidence of residency or eligibility. No final
  participation or prize-eligibility determination was made here.

## Suggested optional demonstration

Our proposal, not an organizer requirement: show one operational incident from
an explicitly selected AWS test environment. Present measured symptoms and source
times, compare likely causes, explain one bounded remedy, obtain deliberate
confirmation, then show the provider outcome and post-action application evidence.
Include one refusal for stale evidence or an out-of-scope action if time permits.

Use a labeled simulated Alexa+ conversational surface if choosing that track,
with the actual implementation boundary clear in the README/video. A simulation
must not be presented as verified AWS mutation or a real Alexa backend connection.
If only read-only observation is built, demonstrate that honestly rather than
claiming autonomous production management. A real bounded AWS action requires the
runtime authority prerequisites; a deadline does not waive them.

No new production service, public endpoint, cloud account operation or contest
registration was performed in this documentation work. Whether to enter and the
final demonstration scope remain user decisions once a runnable path exists.
