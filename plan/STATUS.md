# STATUS — maintained by the manager

_Last updated: 2026-11-23T23:50:21Z (UTC)._

## Overall
Backend core, OTA foundation and release pipeline are done (v0.3.1-rc.2 published as a pre-release).
The report site (R-001) has its first edition built and published; it is in review. The web UI, the OTA admin API, device packaging, the approval flow and the test suite are not started.

## By area
| Area | Done | Next |
|---|---|---|
| Backend | core API, auth/profile, files, trash, shares, search, SSE, usage, admin, OTA updater + release pipeline | R-060/R-061 (approval), R-020, R-021 |
| Frontend | report site (manager-site/), first edition | R-003 (after R-002) |
| Design | report-site design system, glossary, mockups, apple-design review | R-002 (web app screens), R-004 |
| Tests | 84 unit tests | R-030, R-031 |
| Report site | first edition built, checked and published (R-001, in review) | owner review; next report after the next milestone |

## Active tasks
- 2026-11-27 (session 5) · R-031 · tester · main checkout · state: delegated (Bash now allowed) · next: verify scripts/e2e_update.sh runs; also R-021 fake-mirror runtime check; UI-part of R-031 waits for R-026.
- 2026-11-27 (session 4) · tester for R-031/R-021 stopped: Bash denied for compound cd/cargo/self-test calls, nothing run or committed. Tester notes: updater PUBLIC_KEY is compiled in (src/update/mod.rs:29), so the happy-path e2e needs a patched copy build (owner to confirm); UI embedding (R-026) not checked. Owner: allow worker Bash, then restart manager. Issue #6 handled.
- 2026-11-27 end of session 3: tester also denied Bash (R-021 fake-mirror test not run). Everything left (R-021 runtime test, R-030/R-031, R-003 live check, UI tasks R-005..R-013) needs workers with shell permission; owner to allow bash/mktemp/minisign/tar/python3/bun/cargo for subagents, then restart the manager. Waiting on owner: A-004 (icon C or D), manager.md "Choose X" text.
- R-021 committed (24703cf) as `review`: scripts written by an edit-only worker; clippy clean, key-match unit test passes, NOT run (my shell cannot execute the scripts; no docker/shellcheck). Report 2026-11-27-2030-device-packaging deployed, live 200. Next: tester runs the fake-mirror test from the worker's recipe (minisign -G -W throwaway key, --no-system, tamper case) in a session with shell permission; then R-030/R-031. Workers' Bash is denied after 1-2 calls this session, so heavy tasks (R-030, R-031, UI work) need a session where the owner allows it.
- 2026-11-27 (session 3) · A-004 written in APPROVALS (C/D icons), Choose buttons removed from SPEC/APPROVALS (the .claude/agents/manager.md text still mentions Choose X: edit denied to me, owner to fix). DONE and LIVE: /decisions/brand/ shows C and D (A-004 pending), Choose buttons removed; verified by manager (tsc, 11 tests, 14 pages, 374 links); report 2026-11-27-1930-cloud-icons-c-d deployed, live check 200 (site, brand page, report page, C/D svgs). Not viewed in a browser. Note: worker Bash is denied in worktrees and in the main checkout this session (workers edit only; manager builds/commits); the stale branch agent/R-072-brand-cd and the stale worktrees can be pruned by the owner. Next: backlog by priority (R-021 packaging, R-030/R-031 tests, R-003 live check, R-005..R-013 UI) with edit-only workers if Bash stays denied.
- R-072 · designer done (318104e: variants C Hearth, D Roofline in design/brand/, NOT rendered or viewed: shell denies qlmanage/rsvg for me and workers) · next: a session that can render checks C/D at 16/32/256px, then frontend publishes /decisions/brand/ + approval A-004, deploy report. Not done this session.
- R-003 merged (agent/R-003-web-foundation): ui build, lint, 31 vitest pass (verified by manager). NOT run in a browser or against the backend (no browser tool; workers' shell denied temp dirs/servers). Uses bun (ui/bun.lock), not npm: CLAUDE.md says npm, to reconcile. Next: tester live check + screenshots, then file browser (R-007) and R-027.
- R-071 merged (agent/R-071-shared-with-me): cargo test 143+12+155+7 pass, clippy zero warnings. Live curl outstanding; archive download of received folder not done (not in R-071 text).
- R-070 merged (3261515): clippy --all-targets zero warnings, tests pass. Observed once: tests/approval_http.rs `restore_never_reactivates_a_pending_or_rejected_account` failed in a full `cargo test` run, passed on rerun (possible flake under parallel load; tester to investigate). Live /openapi.json check of UploadForm outstanding. Remaining backlog: R-071 backend, R-003 frontend (needs R-002 design, done), R-073 guides.
- B-001 fixed and merged (c7b15a9; cargo test 143 unit + 12 http + 155 chat_fake-mirror pass; 200-run loop of the flaky test could NOT be run: shell loops denied). R-066 stays `review`: remaining cleanup = delete tests/chat_fake.rs (it carries `allow(dead_code, unused_imports)`) and move its tests into `src/chat/` as cfg(test) unit tests (a draft is in worktree .claude/worktrees/agent-ab34f3158d8ec82cd/src/chat/fake_chat_tests.rs, untracked); `git rm` is denied to workers and to me, so someone with permission must do it. Next: that cleanup, then R-070.
- R-066 · tester · committed on agent/manager (c6d9b6f): 24 new integration tests (tests/approval_http.rs, tests/chat_fake.rs), mutation-checked. Found bug B-001 (plan/BUGS.md, chat approve/reject race). tests/chat_fake.rs uses `#![allow(dead_code, unused_imports)]` + #[path] mirroring: violates CLAUDE.md. Delegated fix of both to backend-engineer, branch agent/B-001-decide-race (state: delegated). Next: verify, merge, mark R-066 done, then R-070.
- R-064 merged into agent/manager (from agent/R-064-chat-admin-ops-2): cargo test 142 pass, clippy only known wait_idle (R-070). Known flaky: chat::flow_tests::simultaneous_presses_produce_exactly_one_decision (pre-existing from R-063, ~1 in 3 runs) — must be fixed under R-066. Live curl of /admin/users/delete|restore, /admin/trash/purge outstanding. Next: R-066 (tester) then R-070.
- R-074 + A-003 Choose buttons: merged into agent/manager (from agent/R-074-accept-button-2, 63267c7); tsc, 11 bun tests, build and link check pass; report 2026-11-27-1800-accept-button deployed, live check passed (site, /decisions/, report listed). Not viewed in a browser. R-074 is `review`. Leftover empty branch agent/R-074-accept-button can be deleted (`git branch -d`, denied to workers).
- 2026-11-27 (this session): R-060 verified and merged (cargo test 131 pass; clippy only the known wait_idle warning). A-003 (cloud icon A/B) added to APPROVALS. R-072 · merged into agent/manager (0e53366), report 2026-11-27-1500-brand-variants deployed, live check passed (site, /decisions/brand/, report listed all 200). NOT viewed in a browser (no renderer available to workers; the report says so). Old design/brand/direction-*, README.md, index.html still in the repo: `git rm` was denied to me and the worker; nothing references them. Next: someone with permission removes them; render A/B and add screenshots; await owner answer to A-003.
- 2026-11-27: issues #1-#4 handled and closed (R-001/R-002 done; A-002 approved; A-001 denied). R-060 cooldown removal: committed on agent/R-060-no-cooldown (a04f992, worktree .claude/worktrees/agent-a8c2ccaf357bc5ebc), UNVERIFIED and UNMERGED: cargo is not on PATH in the worker or manager sessions, so build/test/clippy and the curl check have not run. Next: a session with cargo runs them, then merge. R-072: cloud-icon variants A/B committed on agent/manager (4280b24), not rendered or viewed; next: render, frontend publishes /decisions/brand/ page + new approval request, deploy report.
- R-002 · designer · main checkout · state: follow-up delivered (01cc6e5, 5c6ccb9), unviewed in a browser · next: tester/manager looks at mockup at phone width (dark), then R-003 (frontend) and R-071 (backend) can start; no worker active
- R-020 merged into agent/manager (94 tests reported at merge; not re-run in the report session); live curl check still outstanding (needs a running instance).
- Report 2026-11-25-1200-web-app-design built and deployed; live check passed (site 200, report listed, report page 200, screenshots load). Mockup screenshots only; cargo is not available in the report session, so tests were not re-run.

- 2026-11-26 re-check: the live URL could not be checked from this session (DNS for the report host does not resolve here, with or without the sandbox). The earlier live check (2026-11-25) stands; this one is not confirmed. Re-check from a session with network access.
- 2026-11-26 (later): rebuilt (9 pages, 2 reports) and re-ran scripts/deploy_report.sh: upload to the server succeeded, but the live HTTP check still fails here (DNS does not resolve in this session). Published but NOT live-verified in this session.

- 2026-11-26 (final): live check passed from a session with network access: site 200, report listed on /reports/, report page 200, screenshot asset 200 image/png. The earlier DNS failure is resolved. Tests not re-run in the report session (no cargo).

- RESOLVED 2026-11-26: decision pages merged (2d3298b), deployed, /decisions/brand/ and /decisions/admins/ live (200). Owner must answer A-001 and A-002. R-063 still needs a worker with Bash; not re-delegated this session. Earlier note: Bash was denied/needs approval for workers and for the manager in worktrees. Frontend decision pages (branch agent/R-001-decision-pages, worktree .claude/worktrees/decision-pages) are built (bun build/check/tests/links passed per worker) but UNCOMMITTED and not visually checked. Next: owner restores Bash permission; then commit, screenshot at phone width, verify, merge, deploy report.
- R-072 · designer→frontend-engineer · state: directions committed (4f612c5, PNG previews missing: designer had no renderer); A-001/A-002 written in APPROVALS · next: frontend builds /decisions/brand/ and /decisions/admins/ pages in manager-site, deploy
- R-063 · backend-engineer · merged into agent/manager from worktree-agent-a42d510a1b1e1a54b · state: review (cargo test 131 pass; clippy --all-targets: 1 pre-existing warning, `wait_idle` unused, R-070) · next: live curl check (worker could not start the server); live Telegram check waits for the owner's bot token (R-073); then R-064, R-066, R-070, R-003. Bash works for workers again.

- 2026-11-27 (later session): R-060 no-cooldown VERIFIED on its branch (cargo test 84 pass; clippy only the known R-070 idle-code warnings). NOT yet merged into agent/manager: the session's shell was moved into the worktree and cannot run git in the main checkout. Next: from the main checkout run `git merge agent/R-060-no-cooldown`, re-run cargo test (expect 131+), then R-072 render/publish and report deploy. No open GitHub issues.

## Waiting on the owner
See SPEC.md section 6 and the site's approvals queue.

## Owner decisions (2026-11-25, answers to design/app-spec.md section 12)
1. File extensions: **show** them (do not hide by default).
2. Appearance: **keep** the Automatic / Light / Dark control in Settings; the default is Automatic (follow the system).
3. "Shared with me" view: **yes**, add it (the mockup only has "shared by you").
4. Public share page: **yes**, add a language switch.
5. Phone numbers: **yes**, add a country picker.
Record these in SPEC.md and design/app-spec.md, and carry them into R-002 follow-up and R-003.

## Owner requests (2026-11-26) — do these on the next run
The owner's answers to the open questions in SPEC.md section 6 (already recorded there):
1. **Google/Apple sign-in (R-023): interface first.** The owner has developer accounts but no time to fetch credentials now. Build the interface only (see the R-023 row); real credentials come later.
2. **Telegram bot token (R-063): interface first.** Keep building and testing with the fake provider; the live check waits for the token.
3. **Single admin vs several: the owner does not understand it yet and wants an explanation on the report site before deciding.** Do this: (a) write a plain-language explanation page or section on the report site (English + 简体中文, in the `/decisions/` area, linked from the pending decision), covering what changes day to day for each option — who gets the Approve/Reject message, what happens when two admins tap at once (first tap wins, the other sees "already decided by …"), what happens if the one admin loses the phone (re-pairing needs the private `/admin` endpoint and the admin password from the home network), whether admins have equal power (yes, all equal), how pairing works, and a recommendation with the reasoning; use a small diagram or example conversation; (b) add an approval request in `plan/APPROVALS.md` offering the options so the owner can answer with a button; (c) until answered, keep the default (whoever pairs). Source facts: `spec/admin_chat.md` sections 2, 4 (Pairing) and 9.
4. **Brand (R-072): the designer designs a new brand for Cloud Home.** Show 2–3 directions on the report site `/decisions/` with an approval request, then apply the chosen one.
5. **R-073: once the interfaces of R-023 and R-063 are complete, tell the owner how to get the tokens and authorizations from the third parties, and help implement the real wiring.** Publish step-by-step guides on the report site (Telegram BotFather token; Google OAuth client; Apple Sign in with Apple), with the exact config keys, file path and a way to check it works; the guides are public, so no secrets, and the owner must never paste a secret into chat, an issue or the repo (credentials go in `~/.config/cloud_home/`). Add an approval request "Credentials ready?" in `plan/APPROVALS.md`. After the owner confirms, wire the real providers and run the live checks. Details: SPEC.md R-073.
Only the items above plus R-072 and R-073 are new; continue the rest of the plan as before. Deploy the report site after the explanation and brand directions are built.

## Notes for the team (2026-11-26)
The report site works now (https://report.cia1099.cloudns.ch/). If a live check fails, check DNS first: compare `dig +short @8.8.8.8`, `@9.9.9.9` and `@1.1.1.1` for `report.cia1099.cloudns.ch`, and tell the owner which ones fail. Do not change DNS and do not keep retrying the deploy.

## Owner answers on GitHub (2026-11-27)
Read the open issues (#1–#4) and handle them. How to read them:
- **#1, #2 `[DEL R-001]` / `[DEL R-002]`**: the reason says "Well done. I had reviewed it". This means *reviewed and accepted*: mark R-001 and R-002 done. Do **not** delete the report site or the design, and do not remove their code.
- **#3 A-002 approve**: several equal admins. Also: a rejected member may register again, with no limit on the number of tries; drop the cooldown after a rejection (keep the other abuse limits unless they conflict, and ask if they do).
- **#4 A-001 deny**: do not use directions A, B or C. Base the new brand on the report site's cloud icon (header logo and favicon in `manager-site/`) and make it feel more like Apple design, elegant. The issue body has the owner's reference images.
Close each issue after you have acted on it.
- **FIRST THING THIS SESSION (owner, 2026-11-27): publish A-004 and deploy now.** The owner cannot see the new icon designs. Do not wait for anything else and do not hold back for lack of a browser: variants C and D in `design/brand/` were already rendered and checked by the owner's session (C recommended, see the Log). Create approval request A-004 in `plan/APPROVALS.md` (Approve/Deny only; the owner writes the chosen variant in the issue note), replace `/decisions/brand/` with C and D, rebuild the report site, run `scripts/deploy_report.sh`, and check the live URL. Only after it is live go on with the backlog.
- **Owner asked (2026-11-27): deploy a report before the 5-hour limit hits.** The owner wants to see what you did this round. Deploy a report after each verified task (at least every second task or once an hour), not only at the end; a short report is fine. Details: the new paragraph "Never save the report for the end" in your agent file.
- **Owner changed their mind (2026-11-27): no option buttons.** Remove the *Choose A / Choose B* buttons that were added with R-074 (`manager-site/src/shared/issue.ts`, the Decisions renderer, i18n strings, tests, the `plan/APPROVALS.md` format text, the docs and the manager's agent file): a decision card has only **Approve** and **Deny**. When a request lists options (A-003: cloud icon A or B), the owner names the option in the issue's note when approving, and the Deny note says what to change. Reword A-003 so that Approve means "I choose the variant named in my note". Keep the Accept button (R-074). Leave nothing behind (CLAUDE.md idle-code rule), then deploy.
- **Owner asked (2026-11-27): do not stop early.** After these two, keep going through the backlog by priority (R-064, R-066, R-070, R-003, R-071, then R-073 when its interfaces are ready) without waiting for answers. Owner questions go to `plan/APPROVALS.md` and you continue.
- **R-074 (new, owner asked for it): Accept button.** Until it ships, treat a `[CHG R-nnn]` or `[DEL R-nnn]` issue whose text says accepted / reviewed / well done as *accepted*: mark that requirement `done` (never delete work) and close the issue. Also: the owner reported seeing no update on the report site. Render and check the two new cloud icon variants, publish them on `/decisions/` with a fresh approval request, verify and merge R-060 (cargo is on PATH when the owner starts you from their terminal), then build and deploy the site and write the report.

## Log
- 2026-11-27 — owner decisions: (1) workers may run any Bash except the deny list (`Bash(*)` is now in the allow list; the deny list is unchanged), so shell denials should stop; (2) R-031: DO the patched-copy build for the happy-path update test (extra full compile is fine). Restart point: R-021 runtime check against a throwaway signed mirror (incl. tampered artifact), R-031 `scripts/e2e_update.sh`, then R-030, the R-003 live check, R-026 and the UI tasks. Deploy a report after each verified task.
- 2026-11-27 — owner's session rendered R-072 variants C and D with `qlmanage -t -s 400 -o <dir> <svg>` (now allowed, with `sips`): both render. C (house inside a cloud outline, SF Symbols style) reads clearly; D's roof stroke looks detached from the cloud and the door slot is odd; in all variants the tile outline is slightly uneven rather than a clean squircle. They are fine to publish on `/decisions/brand/` as A-004; recommend C. Check 16 px for the favicons yourself. Do not wait for more rendering; continue with the backlog.
- 2026-11-27 — owner's session: deleted tests/chat_fake.rs (cargo test 143+12+7 pass, clippy no warnings); CLAUDE.md and the agent files now say bun (not npm) for the web app; the owner answered A-003 in GitHub issue #5 — process it.
- 2026-11-27 — R-060 (no rejection cooldown) merged into agent/manager by the owner's session: 131 tests pass, clippy only the known `wait_idle` warning (R-070); live curl check of re-registration still outstanding. Earlier note that R-060 was unmerged is obsolete.
- 2026-11-19 — team and SPEC.md created.
- 2026-11-25 — R-020 merged; R-002 follow-up (owner decisions) delivered; R-071 added. R-002 web-app design delivered (abccdae); R-020 verified but unmerged; owner answered the 5 design questions (see Owner decisions).
- 2026-11-25 — R-060/R-061 approval flow merged into agent/manager (84 tests, no new clippy warnings); live curl check outstanding.
- 2026-11-24 — report site design committed; report site first edition built (8 pages, English + 简体中文, light/dark, phone width), first report written from the repo, published.

## Site data (read by manager-site, keep it valid JSON)
The report site renders the Overview from this block. Every field is optional; whatever is missing is shown as an honest
"no data" state. Format: `manager-site/README.md`.

```json site-status
{
  "updated": "2026-11-23T23:50:21Z",
  "chip": { "tone": "accent", "text": { "en": "Backend built · UI not started", "zh": "后端已就绪 · 界面尚未开始" } },
  "headline": { "en": "The backend is built. The web app is next.", "zh": "后端已经建好，界面是下一步。" },
  "lede": {
    "en": "Accounts, files, trash, sharing, search and signed self-updates work. This report site is the first edition; the web app and the device installer have not started.",
    "zh": "账户、文件、回收站、分享、搜索与带签名的自动更新已经可用。本报告网站是第一版；网页界面和设备安装脚本还没有开始。"
  },
  "built_before_plan": { "backend": 10 },
  "roles": {
    "frontend": {
      "working": false,
      "task": { "en": "Idle · last delivered the report site (first edition)", "zh": "空闲 · 最近交付了报告网站（第一版）" },
      "at": "2026-11-23T23:50:21Z"
    },
    "designer": {
      "working": false,
      "task": { "en": "Idle · last delivered the report-site design system and mockups", "zh": "空闲 · 最近交付了报告网站的设计系统与原型" },
      "at": "2026-11-23T17:41:26Z"
    }
  },
  "checks": [
    { "name": { "en": "Unit tests", "zh": "单元测试" }, "state": "ok", "detail": { "en": "84 of 84 passing", "zh": "84 项全部通过" } },
    { "name": { "en": "Release v0.3.1-rc.2", "zh": "发布 v0.3.1-rc.2" }, "state": "ok", "detail": { "en": "Published as a pre-release", "zh": "已作为预发布版发布" } },
    { "name": { "en": "Signed self-update with rollback", "zh": "带回滚的签名自更新" }, "state": "ok", "detail": { "en": "Tested end to end by hand", "zh": "已手动端到端测试" } },
    { "name": { "en": "Report site", "zh": "报告网站" }, "state": "ok", "detail": { "en": "First edition published (R-001, in review)", "zh": "第一版已发布（R-001，待审阅）" } },
    { "name": { "en": "Lint", "zh": "静态检查" }, "state": "warn", "detail": { "en": "9 idle-code sites left (R-070)", "zh": "还剩 9 处闲置代码（R-070）" } },
    { "name": { "en": "Web app", "zh": "网页应用" }, "state": "neutral", "detail": { "en": "Not started (R-003)", "zh": "尚未开始（R-003）" } },
    { "name": { "en": "UI end-to-end tests", "zh": "界面端到端测试" }, "state": "neutral", "detail": { "en": "Not started (R-032)", "zh": "尚未开始（R-032）" } }
  ]
}
```
