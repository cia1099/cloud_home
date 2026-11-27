# APPROVALS — the manager asks, the owner answers

The manager appends a request here whenever it needs your decision (pushing, tagging, releasing,
deploying anything other than the report site, key/secret changes, breaking API changes,
costly or restrictively-licensed dependencies, deleting data or features, unclear requirements).
The report site shows pending requests at the top of the Overview and on `/decisions/`.

**To answer:** tap *Approve* / *Deny* on the report site's Decisions page (opens a pre-filled GitHub issue `[DECISION A-nnn] approve|deny`; submit it, and when the request is a choice between named variants, write the one you pick in the issue note), **or** change the box on the request's first line here and optionally add a note under it.
`[ ]` pending · `[x]` approved · `[!]` denied. The manager reads this file at the start of every cycle.

## Request format
```
### A-001 [!] <short title> — <date> — requested by manager
- What: …
- Why: …
- Risk: …
- If approved: …   If denied: …
- Owner note: DENIED (issue #5; title said "choose A" but the body says deny, so treated as deny). "I want you following Apple SF Symbol style. I hope the brand can present cloud drive at home." Designer redoes the icon: SF Symbols-like cloud glyph (monoline/weight-consistent) that conveys a cloud drive at home (e.g. cloud + house motif).
```
A decision card has only *Approve* and *Deny*. For a choice between named variants, say so in `- What:` and word `- If approved:` as "I choose the variant named in my note"; the deny note says what to change.

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

### A-003 [x] DENIED 2026-11-27 — Cloud icon variant for Cloud Home (R-072) — 2026-11-27 — requested by manager
- What: pick between two refinements of the report site's cloud icon (design/brand/cloud-a.svg "Variant A" and cloud-b.svg "Variant B"), shown on the report site /decisions/brand/.
- Why: you asked to base the brand on the report site's cloud and make it more Apple-like and elegant.
- Risk: low.
- If approved: approve means "I choose the variant named in my note" (A Sky or B Porcelain); I produce the final SVG set from it.   If denied: the note says what to change.
- Owner note: DENIED (issue #5; the title said "choose A" but the body says deny, so treated as deny). "Follow Apple SF Symbol style. I hope the brand can present cloud drive at home." The designer redoes the icon.

### A-004 [ ] Cloud icon: variant C (Hearth) or D (Roofline) — 2026-11-27 — requested by manager
- What: approve one of two SF-Symbols-style "cloud drive at home" icons shown on the report site /decisions/brand/: C Hearth (a house drawn inside the cloud outline) or D Roofline (a cloud wearing a roof, with a doorway cut into its base). Write C or D in the issue note.
- Why: you asked for an SF Symbols style that says "cloud drive at home". C is the more explicit picture and the one the owner's session recommends; D is a bolder silhouette that reads better at 16 px.
- Risk: low. Nothing is applied to the product until you answer.
- If approved: approve means "I choose the variant named in my note". I produce the final SVG set (app icon, maskable, favicon, light/dark, one-colour), usage rules, and apply it to the mockup, sign-in, share page and report-site header.   If denied: the note says what to change and the designer does another round.

### A-005 [ ] Credentials ready? — 2026-11-28 — requested by manager
- What: tell the manager you have put the credentials from the guides on the box, in files readable only by you: the Telegram bot token, the Google OAuth client, and the Apple key with its ids. Step-by-step guides: report site /decisions/guides/.
- Why: the Telegram approval bot can be checked live as soon as its token is in place; the real Google and Apple sign-in is wired only after you confirm.
- Risk: low. You never send a secret to the team; the files stay on your machine (mode 600). Until you approve, nothing is wired and the box ships only the sign-in interface.
- If approved: the backend engineer wires the real providers, runs the live checks (a real approval message on your phone, a real Google and Apple sign-in) and reports the result.   If denied: say in the note what is missing or unclear; the guides are updated.
- zh title: 凭证准备好了吗？
- zh What: 告诉经理：你已把指南里的凭证放到设备上，文件只有你能读。包括 Telegram 机器人令牌、Google OAuth 客户端、Apple 密钥及其标识。分步指南见报告网站 /decisions/guides/。
- zh Why: Telegram 审批机器人的令牌一就位就能现场验证；真正的 Google、Apple 登录要等你确认之后才接入。
- zh Risk: 低。你无需把任何秘密发给团队，文件留在你自己的机器上（权限 600）。你批准之前不会接入任何东西，设备只带登录接口。
- zh If approved: 后端工程师接入真实登录提供方，做现场验证（手机上收到真实的审批消息、真实的 Google 和 Apple 登录），并汇报结果。   zh If denied: 在备注里写明缺什么或哪里不清楚，指南会更新。

### A-006 [ ] Cut stable release v0.3.1 (R-040) — 2026-11-28 — requested by manager
- What: push the work on agent/manager, tag v0.3.1 and publish a stable release (replacing the rc tags).
- Why: R-020, R-021, R-030, R-031, R-060, R-061 and R-066 are implemented and in review; the full test suite passes (190 unit, 39 API, 12 approval, 7 share, 24 browser, 100 OTA e2e). Not yet checked by hand: the web app on a real device, the live Telegram bot, real Google/Apple sign-in (these stay off until credentials exist).
- Risk: medium. A release is public and boxes with auto-update will install it; the OTA rollback path exists and was tested.
- If approved: you merge agent/manager into main yourself or tell me to; I then prepare the release notes and ask once more before the push and tag. If denied: stay on rc tags; the note says what to change.
