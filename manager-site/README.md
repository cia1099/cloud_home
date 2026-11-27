# manager-site: the Cloud Home report site (R-001)

A static, multi-page, public, read-only site generated at build time from files in this repository. bun + Vite + vanilla TypeScript, no runtime dependencies. Output: `report_web/` (git-ignored), copied as a whole to the server by `scripts/deploy_report.sh`.

```
bun install --frozen-lockfile
bun run build          # vite build (hashed JS/CSS) + src/build/generate.ts (every page, hashed media)
bun run preview        # static server on :4173 for report_web/: no rewrites, no SPA fallback, answers Range with 206
bun run dev            # same server, rebuilds when site source, reports/, SPEC.md or plan/ change
bun run check          # tsc --noEmit
bun run test           # parsers and request-link tests (bun test)
bun run check:links    # every href/src resolves, page links end in "/", assets are hashed, files <= 10 MB
bun run build:examples # builds examples/ (sample reports of every component) into report_web_examples/ (never deployed)
```

## How it is built

- Every URL is a real `folder/index.html`; internal links end in `/`; `/latest/` is a real folder with a meta refresh, a link and a JS redirect.
- Every asset has a content hash in its name: the bundle (`assets/main-<hash>.js|css`) and every report media file (`assets/media/<name>-<hash>.<ext>`). Limits: 10 MB per file, 25 MB per report; the build fails otherwise.
- English and 简体中文 are both in the HTML (`<span lang="en">` / `<span lang="zh-Hans">`); the choice is stored in `localStorage` and applied before first paint. aria-labels and alt texts carry `data-al-en` / `data-al-zh`. All UI strings come from one table, `src/shared/i18n.ts`.
- The build fails if any output contains what `scripts/deploy_report.sh` refuses, `/Users/`, or an e-mail address.
- Request buttons are plain links to pre-filled GitHub issues (`src/shared/issue.ts`, tested): `[ADD] `, `[CHG R-012] <title>`, `[DEL R-012] <title>`, `[OK R-012] <title>` (Accept, only on requirements in `review`), `[Q] `, `[DECISION A-003] approve|deny`.

## Data the manager maintains

Nothing is invented: missing data renders an honest empty state.

### `SPEC.md` (parsed, not edited by the site)
Section 3: `### <area>` headings and requirement rows `| R-nnn | P0..P2 | status | **Title** text. Blocked: reason. | owner |`. The title is the first bold phrase, the owner is the first role word in the last cell (so put only the owner there), the blocked reason is the sentence starting `Blocked:`. Section 7 rows (`| date | change |`) become timeline events.
Chinese titles and blocked reasons live in `content/requirements.zh.json` (`{"R-012": {"title": "…", "blocked": "…"}}`); a missing one prints a build warning and shows the English text. A new `###` area heading needs an entry in `content/areas.json`.

### `plan/STATUS.md`: one fenced block
````
```json site-status
{
  "updated": "2026-11-24T15:00:00Z",
  "chip": { "tone": "accent", "text": { "en": "…", "zh": "…" } },
  "headline": { "en": "…", "zh": "…" },
  "lede": { "en": "…", "zh": "…" },
  "built_before_plan": { "backend": 10 },
  "roles": { "frontend": { "working": true, "task": { "en": "R-001 · …", "zh": "…" }, "at": "2026-11-24T15:00:00Z" } },
  "checks": [ { "name": { "en": "…", "zh": "…" }, "state": "ok", "detail": { "en": "…", "zh": "…" } } ]
}
```
````
`tone`: `ok|accent|warn`. Roles: `manager|backend|frontend|designer|tester`; a role without an entry shows "No update recorded yet." `state`: `ok|warn|neutral`. `built_before_plan` keys are area keys from `content/areas.json`.

### `plan/APPROVALS.md`
Under `## Requests` (fenced blocks are ignored): `### A-003 [ ] English title — 2026-12-02 — requested by manager`, `[ ]` pending, `[x]` approved, `[!]` denied. Bullets `- What:`, `- Why:`, `- Risk:`, `- If approved: …   If denied: …`, `- Owner note:`. Chinese: the same keys with a `zh ` prefix (`- zh title:`, `- zh What:`); missing ones fall back to English.

### `plan/burndown.csv`
```
# target=2026-12-31
date,done,total,dropped
2026-11-24,0,35,0
```
Append one row per day (`open = total - dropped - done`). The `# target=` line is optional; without it no ideal-pace line is drawn. Fewer than two rows shows "Not enough history yet".

### `reports/<YYYY-MM-DD-HHMM-slug>/`
The folder name is the report id (date and UTC time, so ids sort chronologically). Files: `report.json`, optional `tests.json`, media files, optional `page.ts` / CSS for a bespoke page.

`tests.json` (the newest one feeds the Overview):
```json
{ "suites": [{ "name": "cargo test (unit)", "passed": 63, "failed": 0, "skipped": 0 }] }
```

`report.json`: every text is `{ "en": "…", "zh": "…" }`; `` `code` `` and `**bold**` are the only markup (everything else is escaped).
```json
{
  "kind": "weekly | milestone",
  "title": {…}, "summary": {…},
  "chip":  { "tone": "ok|accent|warn|neutral", "icon": "check", "text": {…} },
  "cover": { "style": "c1|c2|c3|c-ms", "icon": "swap" },
  "sections": [ { "id": "summary", "title": {…}, "intro": {…}, "caption": {…}, "component": "kpis", "data": {…} } ]
}
```
Components and their `data`:

| component | data |
|---|---|
| `kpis` | `items: [{label, value (number, string or {en,zh}), of?, sub?}]` |
| `text` | `paragraphs?: [text]`, `bullets?: [text]` |
| `diagram` | `kind: "arch"` with `client, binary{name,subtitle,modules}, drive, cloud, links[3]`; a module with `requires: ["R-003"]` is drawn dashed "Not built yet" until all those requirements are `done` in SPEC.md. `kind: "ota"` with `steps: [{icon,title,text,fail?}]` and `rollback` |
| `areas` | none; progress by area from SPEC.md at build time |
| `burndown` | none; from `plan/burndown.csv` |
| `tests` | `note?`; from the report's own `tests.json` (waffle, one square per test) |
| `timeline` | `limit?` (default 8); newest commits and releases up to the report time |
| `callouts` | `items: [{tone: "risk|block|info", title, text}]` |
| `gallery` | `shots: [{src, alt, title, build?, theme?, language?, wide?}]` with `src` relative to the report folder (png, jpeg, webp, svg); lightbox included |
| `compare` | `before, after` (images), `before_label, after_label, alt` |
| `walkthrough` | `steps: [{title, caption, media: {src, poster?, alt?}}]`, `duration_ms?`; `src` may be an animated WebP/PNG or `.mp4`/`.webm` (the server must answer `Range`) |
| `decisions` | `ids: ["A-003"]` from `plan/APPROVALS.md` |

`/product/` collects every `gallery`, `walkthrough` and `compare` section of every report, newest first; with none it shows the honest empty state.

### Bespoke milestone pages
Add `"page": "page.ts"` (and optionally `"page_css": "page.css"`) to `report.json`; sections are then ignored. `page.ts` default-exports `(ctx: BespokeCtx) => string` returning a complete `<main id="main">…</main>`; the shared header, footer, tokens and components stay. `ctx` gives `site` data, `asset(rel)` (hashed URL), `imageSize(rel)`, `section(s)` to render any standard component, and a small `kit` of helpers. The CSS kit for milestone pages (`.ms`, `.ms-hero`, `.device`, `.ms-nav`, `.ms-sec`, `.bignums`, `.walk`/`.wstep`/`.screen`, `.learn`) is in `src/client/style.css`. See `examples/2026-12-03-1500-sample-milestone/`.

`examples/` holds sample reports with made-up data; they only build with `bun run build:examples` and are never part of `report_web/`. `REPORTS_DIR` and `OUT_DIR` override where reports are read and the site is written.
