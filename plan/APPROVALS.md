# APPROVALS — the manager asks, the owner answers

The manager appends a request here whenever it needs your decision (pushing, tagging, releasing,
deploying anything other than the report site, key/secret changes, breaking API changes,
costly or restrictively-licensed dependencies, deleting data or features, unclear requirements).
The report site shows pending requests at the top of the Overview and on `/decisions/`.

**To answer:** tap *Approve* / *Deny* on the report site's Decisions page (opens a pre-filled GitHub issue `[DECISION A-nnn] approve|deny`; submit it), **or** change the box on the request's first line here and optionally add a note under it.
`[ ]` pending · `[x]` approved · `[!]` denied. The manager reads this file at the start of every cycle.

## Request format
```
### A-001 [!] <short title> — <date> — requested by manager
- What: …
- Why: …
- Risk: …
- If approved: …   If denied: …
- Owner note:
```

## Requests

### A-001 [!] Brand direction for Cloud Home (R-072) — 2026-11-26 — requested by manager
- What: pick one of three brand directions shown on the report site /decisions/brand/: A Hearth (house with round window, blue), B Roofbox (amber roof over teal device), C Open Ring (graphite open ring with one blue dot).
- Why: you asked for a new brand; the designer recommends C (quietest, adds no new colour, crisp at 16 px).
- Risk: low; B would add a second brand colour to the UI palette.
- If approved: say which letter in the owner note (or approve = C, the recommendation). I produce the final SVG set (app icon, maskable, favicon, variants), usage rules, and apply it to the mockup, sign-in, share page and report-site header. If denied: tell me what to change and I design a new round.
- Owner note: Denied 2026-11-27 (issue #4): none of A/B/C; evolve the report site's cloud icon, more Apple-like and elegant.

### A-002 [x] One administrator or several? — 2026-11-26 — requested by manager
- What: whether approval requests go to one paired admin or to every paired admin (first tap wins). Explanation with diagram: report site /decisions/admins/.
- Why: you asked for an explanation first. Recommendation: allow several equal admins, but pair only one now; it costs nothing and lets you add a family member later.
- Risk: low; with one admin, a lost phone needs re-pairing from the home network with the admin password.
- If approved: approve = several admins allowed (pairing is still one code at a time; default whoever pairs). Deny = strictly one admin (a second /pair is refused). Until answered the default is "whoever pairs".
- Owner note: Approved 2026-11-27 (issue #3): several equal admins; rejected members may re-register without limit, no cooldown.
