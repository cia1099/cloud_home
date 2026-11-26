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
- R-002 · designer · main checkout · state: follow-up delivered (01cc6e5, 5c6ccb9), unviewed in a browser · next: tester/manager looks at mockup at phone width (dark), then R-003 (frontend) and R-071 (backend) can start; no worker active
- R-020 merged into agent/manager (94 tests reported at merge; not re-run in the report session); live curl check still outstanding (needs a running instance).
- Report 2026-11-25-1200-web-app-design built and deployed; live check passed (site 200, report listed, report page 200, screenshots load). Mockup screenshots only; cargo is not available in the report session, so tests were not re-run.

- 2026-11-26 re-check: the live URL could not be checked from this session (DNS for the report host does not resolve here, with or without the sandbox). The earlier live check (2026-11-25) stands; this one is not confirmed. Re-check from a session with network access.
- 2026-11-26 (later): rebuilt (9 pages, 2 reports) and re-ran scripts/deploy_report.sh: upload to the server succeeded, but the live HTTP check still fails here (DNS does not resolve in this session). Published but NOT live-verified in this session.

- 2026-11-26 (final): live check passed from a session with network access: site 200, report listed on /reports/, report page 200, screenshot asset 200 image/png. The earlier DNS failure is resolved. Tests not re-run in the report session (no cargo).

- RESOLVED 2026-11-26: decision pages merged (2d3298b), deployed, /decisions/brand/ and /decisions/admins/ live (200). Owner must answer A-001 and A-002. R-063 still needs a worker with Bash; not re-delegated this session. Earlier note: Bash was denied/needs approval for workers and for the manager in worktrees. Frontend decision pages (branch agent/R-001-decision-pages, worktree .claude/worktrees/decision-pages) are built (bun build/check/tests/links passed per worker) but UNCOMMITTED and not visually checked. Next: owner restores Bash permission; then commit, screenshot at phone width, verify, merge, deploy report.
- R-072 · designer→frontend-engineer · state: directions committed (4f612c5, PNG previews missing: designer had no renderer); A-001/A-002 written in APPROVALS · next: frontend builds /decisions/brand/ and /decisions/admins/ pages in manager-site, deploy
- R-063 · backend-engineer · merged into agent/manager from worktree-agent-a42d510a1b1e1a54b · state: review (cargo test 131 pass; clippy --all-targets: 1 pre-existing warning, `wait_idle` unused, R-070) · next: live curl check (worker could not start the server); live Telegram check waits for the owner's bot token (R-073); then R-064, R-066, R-070, R-003. Bash works for workers again.

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
Two observations from the owner's session about the report site's DNS. Read them before you touch DNS or treat a live check as failed.
1. **How the domain resolves.** `cia1099.cloudns.ch` is a free subdomain of ClouDNS, so its zone is hosted on ClouDNS and ClouDNS answers first. The NS records at the zone's top are only a list, not a delegation; only `www` is really delegated to Cloudflare (`dylan`/`lindsey.ns.cloudflare.com`). ClouDNS rejects new NS records for other names ("Invalid record"), so a new subdomain is created as a **CNAME to `www.cia1099.cloudns.ch`** in ClouDNS (never a wildcard NS, never hard-coded Cloudflare IPs), plus the matching record in Cloudflare. `report.cia1099.cloudns.ch` already works this way. Before it was fixed, Google/Quad9/Alibaba DNS answered "does not exist" while 1.1.1.1 resolved it; this is the "DNS does not resolve in this session" failure seen in earlier live checks.
2. **Rules for DNS and live checks.** Do not delete the `www` record in Cloudflare (`report` depends on it), do not enable DNSSEC, and do not remove the top-level NS records in ClouDNS. You cannot change DNS yourself (no ssh/dig access to the registrar panels), so if a live check fails, compare several resolvers before concluding anything (`dig +short @8.8.8.8 report.cia1099.cloudns.ch A`, then `@9.9.9.9`, `@1.1.1.1`) and tell the owner which ones fail instead of retrying the deploy. A deploy that uploaded but could not be live-checked is "published, not verified", never "live".

## Log
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
