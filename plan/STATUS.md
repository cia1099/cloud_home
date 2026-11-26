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

## Waiting on the owner
See SPEC.md section 6 and the site's approvals queue.

## Owner decisions (2026-11-25, answers to design/app-spec.md section 12)
1. File extensions: **show** them (do not hide by default).
2. Appearance: **keep** the Automatic / Light / Dark control in Settings; the default is Automatic (follow the system).
3. "Shared with me" view: **yes**, add it (the mockup only has "shared by you").
4. Public share page: **yes**, add a language switch.
5. Phone numbers: **yes**, add a country picker.
Record these in SPEC.md and design/app-spec.md, and carry them into R-002 follow-up and R-003.

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
