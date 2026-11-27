# Cloud Home design system

Status: **Part 1 (report site, R-001) is complete. Part 2 (the web app, R-002) is at the end of this file** (section "Part 2. Web app"); the report site and the web app share the colour, type, radius and motion tokens of Part 1 so they feel like one product. The app mockups are in `design/mockups/app/` and the implementation notes for the frontend engineer in `design/app-spec.md`.

Visual direction (R-004): Apple web style, like iCloud. Calm, spacious, content-first, one accent colour, restrained motion, system fonts only, original icons. The mockups in `design/mockups/report-site/` are the reference implementation: `assets/site.css` holds every token below as a CSS custom property, `assets/site.js` renders every component from plain data. Copy token names and component markup from there.

How to open the mockups: serve the folder with any static server and open the root, for example `cd design/mockups/report-site && python3 -m http.server 8080`, then http://localhost:8080/. Every link is a folder URL ending in `/` and works on a server with no rewrites. Review switches (query string, remembered per session): `?lang=en|zh`, `?theme=light|dark|auto`, `?state=real|sample`, and `/product/?pop=1` for the populated product page.

---

# Part 1. Report site

## 1. Principles

1. **Content first.** The numbers, charts and pictures are the interface. Chrome is a thin translucent bar.
2. **One accent.** Blue means "interactive or in progress". Green means done or healthy, amber means needs attention or planned, red means blocked or failed, purple means in review. Nothing else is coloured.
3. **Honest.** Every number on the real site is computed from real data (`SPEC.md`, `plan/STATUS.md`, `plan/APPROVALS.md`, test output, the git log, `reports/<id>/report.json`). The mockups carry sample numbers (212 tests, 3 Dec 2026, the 30-open burn-down and so on) only so the populated layouts can be reviewed. They are labelled: a "Sample" strip under the header whenever `?state=sample`, and on pages that are templates. The real state (`?state=real`, the default) shows the project's true state on 24 Nov 2026: 63 passing unit tests, release v0.3.1-rc.2 published as a pre-release, signed OTA self-update with automatic rollback tested end to end, no web UI. Never show a stand-in screenshot as if it were the product.
4. **Calm motion that is optional.** Motion explains (a chart fills, an update step lights up); it never carries meaning alone, and with `prefers-reduced-motion` every page shows its finished state.
5. **Public.** No secrets, local paths, server details or owner contact data anywhere. Issue links contain only ids and text already shown on the page.

## 2. Colour tokens

All tokens are CSS custom properties on `:root` (light) and `[data-theme=dark]`. Contrast ratios are computed (WCAG 2.x) against the surface the token is used on.

### Neutrals and surfaces

| Token | Light | Dark | Use | Contrast (text tokens) |
|---|---|---|---|---|
| `--bg` | #fbfbfd | #000000 | page | |
| `--bg-alt` | #f5f5f7 | #0b0b0d | footer band | |
| `--surface` | #ffffff | #1c1c1e | cards, tab bar content | |
| `--surface-2` | #f5f5f7 | #2c2c2e | inset fields, chips, hover | |
| `--text` | #1d1d1f | #f5f5f7 | primary text | 16.3:1 on bg / dark on #000 19:1 |
| `--text-2` | #6e6e73 | #a1a1a6 | secondary text, captions | 4.91:1 on bg, 4.66:1 on surface-2 / dark 8.16:1 on bg, 6.61:1 on surface |
| `--text-3` | #86868b | #8e8e93 | **decorative only** (dotted ideal line, search glyph). 3.51:1, never used for text | |
| `--line` | rgba(0,0,0,.09) | rgba(255,255,255,.12) | hairlines, card ring | |
| `--line-strong` | #d2d2d7 | #48484a | dashed outlines, control borders | |
| `--glass` | rgba(251,251,253,.72) | rgba(28,28,30,.72) | header and tab bar fill (with 20px blur) | |
| `--track` | #e8e8ed | #2c2c2e | empty part of bars and rings | |

### Accent

| Token | Light | Dark | Use | Contrast |
|---|---|---|---|---|
| `--accent` | #0066cc | #2997ff | links, text buttons | 5.39:1 on bg / 6.96:1 on #000, 5.64:1 on surface |
| `--accent-fill` | #0071e3 | #0a84ff | chart lines, active segment, focus-adjacent fills | non-text >= 3:1 |
| `--btn-fill` | #0071e3 | #0071e3 | primary button background, white label | 4.70:1 (dark theme keeps #0071e3 on purpose: white on #0a84ff is only 3.65:1) |
| `--accent-tint` | rgba(0,113,227,.10) | rgba(41,151,255,.16) | secondary button, selected row | accent text on tint 4.83:1 |
| `--focus` | #0071e3 | #64b5ff | focus ring colour | |

### Status colours (see section 3)

| Status | `--st-*-fg` light / dark | `--st-*-bg` light / dark | Text contrast | Chart fill `--c-*` light / dark |
|---|---|---|---|---|
| todo | #5c5c61 / #a1a1a6 | #ececf0 / #313134 | 5.64 / 5.04 | #d2d2d7 / #6e6e73 |
| doing | #0a5fc4 / #64b5ff | #e4effc / #1e3247 | 5.24 / 5.99 | #0071e3 / #2997ff |
| review | #7b3fb0 / #c79bf0 | #f0e7f8 / #392744 | 5.50 / 6.06 | #8e4fc4 / #bf5af2 |
| done | #1b7339 / #4cd77a | #e0f3e6 / #203d28 | 5.10 / 6.43 | #1b9a47 / #30d158 |
| blocked | #bf2f1a / #ff8a7a | #fce8e4 / #452323 | 4.89 / 6.04 | #d93a22 / #ff453a |
| dropped | #6e6e73 / #98989d | transparent, 1px outline | 4.91 / 6.0 | n/a (excluded from totals) |
| warn (decisions, attention) | #8a4b00 / #ffc971 | #fff3df / #33260f | 6.20 / 9.72 | dot #e08a00 / #ff9f0a |

Rules: chart fills only need 3:1 as graphics (`--c-done` light is 3.65:1 on white, `--c-blocked` 4.59:1, `--c-todo` is deliberately the empty-track grey and its legend swatch has a 1px outline). Status is never colour alone: every chip carries an icon glyph and a word.

### Bespoke milestone pages

`.ms` re-declares the surface tokens as a fixed dark "cinema" theme (`--bg #0b0b0f`, `--surface #17171c`, `--text-2 #a1a1a6` = 7.64:1, accent `#64b5ff`). It is the one place the page ignores the user's light/dark choice; it is announced in the design as intentional and the shared header still follows the chosen theme.

## 3. Status mapping

| Status (SPEC.md) | Word EN / 简中 | Icon in chip | Colour | Counts as |
|---|---|---|---|---|
| todo | To do / 待办 | empty circle | neutral grey | open |
| doing | In progress / 进行中 | half-filled circle | blue | open |
| review | In review / 审核中 | magnifier | purple | open |
| done | Done / 已完成 | filled circle with check | green | done |
| blocked | Blocked / 受阻 | circle with pause bars | red | open, and the reason is shown under the title |
| dropped | Dropped / 已取消 | circle with dash, struck-through label | outline only | excluded from every total |

Priority `P0 / P1 / P2` is a 30x22 outlined badge (P0 has a heavier outline). It does not use colour. Meaning on hover/title: must / should / nice.

Overall percentage = done / (total - dropped). Area bars use the same formula per area.

## 4. Typography

System stack only, no remote fonts.

```
--font:    -apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display", "Helvetica Neue",
           "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", "Noto Sans CJK SC", system-ui, sans-serif
--font-zh: -apple-system, BlinkMacSystemFont, "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei",
           "Noto Sans CJK SC", system-ui, sans-serif
--mono:    ui-monospace, "SF Mono", SFMono-Regular, Menlo, Consolas, monospace
```

`:lang(zh-Hans)` switches to `--font-zh`, letter-spacing 0, `line-break: strict`. For the web app's Traditional Chinese use `:lang(zh-Hant)` with `"PingFang TC", "PingFang HK", "Microsoft JhengHei", "Noto Sans CJK TC"` ahead of the SC faces. `<html lang>` is always set from the active language (`en`, `zh-Hans`, `zh-Hant`) so the right glyph variants render.

| Role | Size / line | Weight | Tracking | Notes |
|---|---|---|---|---|
| Display (page title) | clamp(34px, 5.6vw, 64px) / 1.05 | 700 | -0.03em | zh: 1.18 line, -0.01em |
| Milestone hero | clamp(40px, 8vw, 96px) / 1.02 | 700 | -0.04em | gradient text, zh 1.15 |
| Title (section) | clamp(24px, 3vw, 32px) / 1.15 | 700 | -0.022em | |
| Heading 3 | 21px / 1.25 | 600 | -0.018em | |
| Lede | clamp(18px, 2vw, 21px) / 1.4 | 400 | | `--text-2`, max 36em |
| Body | 17px / 1.47 (16px on phone) | 400 | -0.011em | zh line 1.65 |
| Sub | 15px | 400 | | `--text-2` |
| Small, caption | 13px | 400 | | `--text-2`; minimum text size is 11.5px (badges) |
| Eyebrow | 13px | 600 | .01em | |
| KPI value | 44px (36 phone, 32 at 430px) | 700 | -0.035em | tabular numerals |
| Ring value | clamp(56px, 8vw, 84px) | 700 | -0.04em | |
| Big number | clamp(44px, 7vw, 88px) | 700 | -0.05em | milestone only |
| Hash / ids | 12 to 13px mono | 500 to 600 | 0 | |

Numbers that change use `font-variant-numeric: tabular-nums`. Headings use `text-wrap: balance`, paragraphs `text-wrap: pretty`.

## 5. Spacing, layout, radii, elevation

- Base unit 4px; common steps 8, 12, 16, 20, 24, 32, 40, 56, 72, 112.
- `.wrap` = `min(1120px, 100% - 48px)` (100% - 32px at <= 860px); `.read` = 720px; header content up to 1280px.
- Card padding 24px (20px phone). Grid gap 16px. Section top padding 56px (40px phone). Header 52px tall, phone tab bar 56px + safe area.
- Breakpoints: 1000px (4-up grids become 2-up, table-of-contents hides), 860px (phone/tablet layout: tab bar, stacked grids, sheet), 430px (KPI value shrinks).
- Radii: `--r-s 8px` (inputs, thumbs), `--r-m 12px` (rows, callouts), `--r-l 20px` (cards), `--r-xl 28px` (feature cards, diagrams, players, sliders), `--r-pill 980px` (buttons, chips, segmented control), 6px for ids and priority badges.
- Elevation: `--sh-1` = 1px ring + 0 1px 2px at 4% (cards, rest). `--sh-2` = 0 10px 40px at 8% + ring (hover, floating). `--sh-3` = 0 24px 80px at 18% (lightbox art). Dark theme drops the soft drop shadow at rest and keeps the 1px ring.
- Blur (functional layer only, never content): header, tab bar, milestone nav, sheet, lightbox use `saturate(180%) blur(20px)` (lightbox 24px). With `prefers-reduced-transparency` they become opaque.

## 6. Motion

| Token | Value | Use |
|---|---|---|
| `--ease` | cubic-bezier(.22,.61,.36,1) | enters, reveals, hovers |
| `--ease-io` | cubic-bezier(.65,0,.35,1) | line draws, travelling dots |
| `--ease-spring` | cubic-bezier(.34,1.35,.64,1) | waffle squares |
| `--d1` | .18s | hover, press |
| `--d2` | .35s | sheet, lightbox, card lift |
| `--d3` | .7s | scroll reveal (fade + 16px rise, 70ms stagger via `--rd`) |
| `--d-chart` | 1.1s | ring and bar fills (140 / 90ms stagger per segment) |

Other timings: counters 1.1s ease-out quart; burn-down line 1.6s (`--ease-io`), area 1s after .9s, points after 1.3s; OTA diagram 1.6s per step (8 steps = 12.8s loop); architecture packets 2.6s loop; walkthrough 3.4s per step; before/after intro sweep 1.1s from 92% to 50%.

Everything reveals once, when it scrolls into view (IntersectionObserver). Hover and press are scale .97 / translateY(-3px) at most.

**Reduced motion (`prefers-reduced-motion: reduce`)**: reveals show immediately; counters print the final number; bars, ring, burn-down and waffle render complete; `.flow` nodes are the tinted "done" colour with no travelling dot; architecture packets are hidden; walkthrough starts paused and never autoplays (the user can still press Play or choose a step, and scenes swap without a fade); the before/after slider starts at 50% with no sweep; the milestone hero drift and device tilt stop; pulsing status dots stop; smooth scrolling is off. Implementation: a global rule shrinks all durations to .001s and iteration count to 1, plus the explicit overrides listed in `site.css` under the media query. JavaScript also checks `matchMedia` for the counters, slider sweep and player.

## 7. Iconography

Original line icons on a 24px grid, 1.75px stroke, round caps and joins, `currentColor`, drawn in `site.js` (`P` map). No Apple glyphs and no SF Symbols are embedded (SF Symbols may not be used outside Apple platforms). Sizes: 20px in controls, 16px inside buttons, 12px inside chips, 24px on the phone tab bar. Status glyphs are 12px (`SG` map). The logo is an original rounded-square with a cloud and a blue gradient. Icons are decoration (`aria-hidden`) and always sit next to a visible or screen-reader label.

## 8. Language, theme, state handling

- `boot.js` runs in `<head>` before paint and sets `data-l` (`en` or `zh`), `lang`, `data-theme` and `data-state` on `<html>`, so there is no flash. Language order: `?lang`, stored choice, `navigator.language` (`zh*` gives Simplified, anything else English). Theme: `?theme`, stored choice, then `prefers-color-scheme`.
- Bilingual copy is rendered as two sibling spans, `<span lang="en">` and `<span lang="zh-Hans">`; CSS shows the one that matches `html[data-l]`. The real site can instead emit one language per file if it prefers, but must keep the same switch behaviour and URL structure (one set of URLs, no `/zh/` prefix).
- The language control is a two-button segmented control "EN | 中文" with `aria-pressed`, each button has its own `aria-label` ("English", "简体中文"), 40x44px minimum on touch, keyboard operable (Tab, Space/Enter). The theme control cycles Auto, Light, Dark and exposes the current mode in its label; on phone both live in the header, and in the "More" sheet as well.
- `data-state` (`real` default, `sample` for review) only exists in the mockups: the real site has no sample state. Elements marked `.only-real` / `.only-sample` show the matching variant.

## 9. Components

For each component: what it is, the data it needs (so the frontend builds it from `reports/<id>/report.json`, `SPEC.md`, `plan/*.md`, git), states, accessibility. Class names are those in `site.css`.

### 9.1 Header and navigation (`.hdr`, `.nav`, `.tabbar`, `.sheet`)
- Desktop: 52px glass bar. Left: logo + "Cloud Home" + "Report". Centre: six pill links (Overview, Reports, Timeline, Requirements, Product, Decisions), current page has `aria-current="page"` and a filled pill; Decisions shows an amber count badge when pending > 0 (badge also has an `aria-label`). Right: **Ask a question** (secondary button with `[Q] ` link), language switch, theme button.
- Phone (<= 860px): header keeps logo, a 44px round Ask button (icon only, accessible name "Ask a question"), language switch and theme button; a 5-item tab bar sits at the bottom (Overview, Reports, Requirements, Decisions, More); More opens a bottom sheet with Timeline, Product, Ask a question, Language and Appearance. Header at 390px: 0 overflow, all targets >= 44px.
- Data: `pending_count` (number of open approvals), current path.
- A "Skip to content" link is the first focusable element.

### 9.2 Pending-decisions banner (`.pend`)
First block of the Overview. Amber when `pending_count > 0`: pulsing dot, "N decisions are waiting for you", the title of the oldest, a Review button to `/decisions/`. Green "Nothing is waiting for your decision" when zero. Data: `approvals[]` where `state = pending` (`id`, `title`, `requested_at`). Pulse stops with reduced motion.

### 9.3 Hero with ring (`.hero`, `.ring-wrap`)
Overall status chip, display title, lede, meta line (updated time, link to latest report) and the **progress ring**: one SVG circle per status (done, review, doing, blocked; todo is the track), start at 12 o'clock, 1.2% gaps, segments fill in sequence. Centre: percentage and caption. Legend below with counts. Data: counts per status from SPEC.md; `status_headline`, `status_lede` strings from the manager's report or STATUS.md "Overall". SVG has `role="img"` and a text alternative ("15% of requirements done").

### 9.4 KPI card (`.card.kpi`)
Icon tile, label, big number with optional "of N", sub-line with optional delta. Data: `{label, value, of?, sub?, delta?}` where value is a number (animated count) or short string (for example `v0.3.1-rc.2`). Four across at desktop, two across at phone. Never invent a delta: omit it when there is no previous report.

### 9.5 Progress bars (`.bars`, `.bar`)
One row per area: name, "N of M planned done", and a segmented bar in status colours (done, review, doing, blocked, then the empty track as todo). Optional `built` count shown in green text ("10 built before the plan"). Bars grow from the left in sequence. Data: per area `{id, name_en, name_zh, counts{status}, built?}`. `role="img"` with an aria-label such as "Backend: 0/7".

### 9.6 Burn-down (`#burn`, `.chart`)
Line of open requirements per day over the plan window, a dotted ideal line to the target date, soft area fill, one focusable point per day showing a tooltip line ("Dec 3 · 30 requirements still open") that is also an `aria-live` region; a "View as table" disclosure gives the same data as a table. The SVG is drawn at the container's pixel width and redrawn on resize so its 12px axis labels stay 12px on phones. A single data point (first day of a plan) is a valid state: the point plus the ideal line and the note "first day of the plan". Data: `[{date, open}]`, `start`, `target`.

### 9.7 Test results: waffle and checks (`.waffle`, `.checks`)
Waffle: one rounded square per test (green pass, hatched red outline fail, grey skip); squares pop in along the grid with an 12ms stagger; 18px cells under 100 tests, 12px above. Beside it the large count and "passed . failed . skipped". `role="img"` with a full text alternative. Checks list: status dot (icon + colour) / name / right-aligned result. Data: `tests{passed, failed, skipped, suite}`, `checks[{name_en, name_zh, status: ok|warn|neutral, detail_en, detail_zh}]`. A failing test must also appear as text in the report, never only as a red square.

### 9.8 Role card (`.role`)
Avatar (initial on a role colour), role name, working/idle state with a dot (pulse only when working), current task text (`R-id` + title), relative time. Data: `roles[{id, name_en, name_zh, working, task_en, task_zh, at}]` from the latest log entry per role; idle roles say what is next. Avatar colours are identity, not status, and are never the only identifier.

### 9.9 Report card and feature card (`.rcard`, `.feature`, `.cover`)
Gradient cover with a glyph, status chip, date, title, one-line summary, "Read" affordance. Feature variant (newest report) is a two-column card. The whole card is one link. Data: `{id, kind: weekly|milestone, date, status, title_en, title_zh, summary_en, summary_zh, cover}`. Cover gradients are the tokens `.cover`, `.c2`, `.c3`, `.c-ms`; milestone covers are free.

### 9.10 Timeline item (`.tl`, `.ev`)
Vertical line with a typed pin (commit, release, report, decision, status), month headers, an event card: title, type label, date, **commit hash chip** (links to the GitHub commit), optional tag chip, optional link to the report, requirement or decision. Filter chips by type. Data: `events[{date, type, title_en, title_zh, hash?, tag?, ref?}]`; the real build derives commits from `git log` (hash, date, subject) and releases from tags. Times are UTC and stated on the page. Used full-size on `/timeline/` and compact inside a report.

### 9.11 Requirement row and action buttons (`.req`, `.acts`)
Grid row: id (mono), title (plus blocked reason), priority badge, status chip, owner, and two ghost buttons **Change** and **Remove**. Grouped by area with a mini progress bar, filter chips by status with counts, a search field (id and text, both languages), a totals strip, and a **New requirement** button at the top. On phone a row is a tap-to-expand item and the two actions appear as 44px buttons. Each button is a plain link (`target=_blank`, `rel=noopener`) to `https://github.com/cia1099/cloud_home/issues/new?title=<encoded>&body=<encoded>` with title prefixes `[ADD] `, `[CHG R-012] <title>`, `[DEL R-012] <title>` and a short body skeleton. Data: `requirements[{id, priority, status, area, title_en, title_zh, owner, blocked_reason_en?}]` parsed from SPEC.md. Empty result state: "No requirement matches." with a hint.

### 9.12 Decision card (`.dcard`) and history
Large card per pending approval: id chip, who and when, title, definition list (What, Why, Risk, If approved, If denied), and a primary **Approve** and neutral **Deny** button (48px on phone) plus a note that nothing is sent until the owner presses Submit on GitHub. Links use `[DECISION A-003] approve` / `deny`. A three-step explainer sits above. History list: id, title, date, note, decided chip (Approved green with check, Denied red with cross). Empty state: "You are all caught up". Data: `approvals[{id, title_en, title_zh, requested_at, what, why, risk, if_approved, if_denied, state: pending|approved|denied, decided_at?, note?}]` from `plan/APPROVALS.md` (the real file is English only; zh strings are produced by the manager when it writes the entry).

### 9.13 Diagram: architecture (`data-diagram="arch"`, `.arch`)
Three columns on desktop (clients, `cloud_home` binary containing modules, external drive) plus GitHub releases below the binary; stacked vertically on phone. Modules are cards with an icon, title, description and a status badge; a module that does not exist yet (`status: planned`) is **dashed** with an amber "Not built yet" badge, so the diagram cannot overstate the product. Links between nodes carry a short label and a travelling dot (hidden in reduced motion). Data:

```json
{ "client": {"icon":"phone","title":{"en":"…","zh":"…"},"text":{…}},
  "binary": {"name":"cloud_home","subtitle":{…},"modules":[{"icon":"window","title":{…},"text":{…},"status":"built|planned","badge":{…}}]},
  "drive":  {"title":{…},"text":{…},"modules":[…]},
  "cloud":  {"icon":"cloud","title":{…},"text":{…}},
  "links":  [{"en":"pages and calls","zh":"页面与请求"}, …] }
```
Status must come from SPEC.md: a module is `planned` while its requirement (for example R-003 and R-026 for the embedded web app) is not done.

### 9.14 Diagram: OTA update flow (`data-diagram="ota"`, `.flow`)
Eight numbered steps (check release, download, verify minisign signature, self-test, switch `current` symlink, restart, health check, done or rolled back) in a 4 x 2 grid at desktop, a vertical rail on phone. Each step: icon node, title, one sentence, and an optional red **failure outcome** line with an undo icon ("Not healthy: rolls back automatically"), so the failure path is text, not just colour. Animation: nodes light in sequence with a travelling dot, looping every `steps x 1.6s`; reduced motion shows all nodes in the finished tint. Below it a dashed red "If any step fails" strip. Data: `steps[{icon, title, text, fail?}]` (each with en and zh).

### 9.15 Callout (`.callout`)
Left-bar callout in three tones: risk (amber), block (red), info (blue), each with an icon, a heading and text. Data: `{tone, title, text}`. Blocked items name who is needed ("Waiting on the owner").

### 9.16 Gallery and lightbox (`.gal`, `.shot`, `.lb`)
Responsive grid (3 columns, 2 at tablet, 1 on phone; `wide` items span two). Each tile is a real `<button>` with the image, a caption and an optional "Sample image" tag. Lightbox: full-screen dark glass dialog (`role=dialog`, `aria-modal`), counter, close, previous and next (44px round buttons), caption (aria-live); opening makes the page `inert`, locks scroll, traps Tab, returns focus to the opener on close. Keys: Escape, Left, Right. Touch: swipe left or right to move, swipe down to close, tap outside to close, safe-area insets respected. Data: `shots[{src, alt_en, alt_zh, title_en, title_zh, build, theme, language, wide?}]`; real images need `alt` text, width and height attributes (no layout shift), and `loading=lazy`. Images <= 10 MB each.

### 9.17 Before and after slider (`.ba`)
Two stacked layers with a clip and a 44px handle. The control is a native `<input type=range>` laid over the image (opacity 0) so mouse and touch drag, arrow keys (step 2), Home and End, and screen readers all work; `aria-valuetext` reads "Before 46%, after 54%". Focus draws a 3px ring around the slider and the handle. Labels "Before"/"After" sit in dark pills. The slider sweeps from 92% to 50% once when it first scrolls into view, unless reduced motion or the user has already touched it. Data: `{before_src, after_src, before_label, after_label, alt}`.

### 9.18 Walkthrough player (`.player`)
16:9 stage with scenes (cross-fade), 44px Play/Pause button, progress track, "n / N" counter, live caption and a list of step buttons (each 44px high, `aria-current` marks the active one, Left and Right move between steps). Autoplays only when 60% visible and reduced motion is off; pauses when scrolled away or the tab is hidden; a user choice (pause, step) stops autoplay for good. Last step offers Replay. The real site can use animated WebP or a short MP4 per step (video must answer `Range` with 206); the step list and captions are the accessible transcript. Data: `{steps[{title, caption, media(src|poster)}], duration_ms}`.

### 9.19 Empty state (`.emptyframe`)
Dashed frame, a dashed wireframe placeholder with a shimmer (static in reduced motion), a clear statement and what will appear. `/product/` today: "The web app is not built yet." / "Screenshots and walkthroughs will appear here.", three short notes (Screenshots, Walkthroughs, Before and after) and a link to the web-app requirements. Rule: every list-type page has an honest empty state that says what is missing and what happens next; never a stand-in picture.

### 9.20 Chips, badges, buttons, forms
- Status chip 24px high, pill, icon + word. Neutral, accent, ok, warn variants for non-status labels.
- Buttons: pill, 36px (44px on phone and on `pointer: coarse`), variants primary (`--btn-fill`, white text), secondary (accent text on tint), neutral (surface-2 with ring), ghost (text only, red on hover for Remove). Press scales .97. Focus ring 3px `--focus`, 2px offset, on every interactive element. Disabled is not used on the site (links only).
- Filter chip: 32px (44px touch), `aria-pressed`; selected = inverted text colour.
- Search: pill input with leading glyph, localized placeholder.
- Sample tag (`.sample`): 11.5px outlined label used on template and sample content.

### 9.21 Milestone page kit (bespoke pages, `.ms-*`)
A milestone report is a hand-built page that reuses the header, footer, tokens and components but may add its own layout. Parts demonstrated in `reports/m1-first-ui/`: full-height hero with drifting colour wash and a tilting device frame (tilt settles on scroll), sticky section pill nav with scroll-spy, big-number row (counters, gradient numerals), scroll-driven walkthrough (sticky device on the left, step texts on the right, the screen swaps as each step crosses the middle of the viewport; on phone the device sticks above the text), before/after slider, a 3 x 2 matrix (three languages x light and dark), a "what we learned" trio, and the pager. All content on that page is sample and placeholder art, clearly marked. Routine reports use the standard template (`rhero`, `rlayout`, `toc`, `rsec`) composed from components above in the order given by `report.json`.

## 10. Page inventory and states

| URL | Purpose | States designed |
|---|---|---|
| `/` | Overview: pending banner, hero + ring, KPIs, area bars + burn-down, roles, build and test health, latest report | real (true state today), sample (populated), loading is not needed (static) |
| `/reports/` | Index: featured newest, all cards, templates and samples | one real report today |
| `/reports/<id>/` | Standard report (components) or bespoke | `2026-11-24-backend-foundation` (real, standard), `m1-first-ui` (bespoke template) |
| `/latest/` | Redirect page to the newest report (meta refresh + link, bilingual) | |
| `/timeline/` | Events with commit hashes, filters | real git history; sample |
| `/requirements/` | Read-only SPEC with live status, filters, search, New requirement, Change, Remove | all todo today; sample mix; no-match |
| `/product/` | Gallery, walkthroughs, before/after | **empty (real)**; populated preview with sample images |
| `/decisions/` | Pending approvals with Approve and Deny, history | none pending (real); two pending + history (sample) |

Error and offline: the site is static, so there is no loading or API error state. A broken image shows its `alt` text on a `--surface-2` block; the 404 page is the web server's. Offline, a visited page keeps working because nothing is fetched at runtime.

## 11. Responsive behaviour

- **Phone (<= 860px, checked at 390px)**: single column, tab bar + More sheet, header with 44px icon controls, tap-to-expand requirement rows with 44px Change and Remove, stacked diagrams (architecture top to bottom with vertical links, OTA as a vertical rail), step list under the player stacked one per row, lightbox with swipe, charts redrawn at pixel width, sticky device above the milestone walkthrough.
- **Tablet (861 to 1000px)**: header nav stays, grids go two-up, table of contents hidden, requirement owner column hidden.
- **Desktop (> 1000px)**: four-up KPIs, two-up chart cards, sticky table of contents on reports, full requirement row.
- Long Chinese strings: every card and row uses `minmax(0,1fr)` columns, `overflow-wrap: break-word`, `line-break: strict` for zh; verified at 390px with the longest zh strings (requirement titles, callouts, diagram text) and no horizontal scroll.

## 12. Accessibility

- Contrast: all text tokens meet 4.5:1 on the surfaces they are used on (values in section 2); large text and graphics meet 3:1. `--text-3` is decoration only. `prefers-contrast: more` darkens `--text-2` and strengthens lines.
- Focus: visible 3px ring on every interactive control, plus a ring on the slider container; skip link first; the lightbox traps focus and restores it; the More sheet focuses its first item and closes on Escape.
- Targets: >= 44px on phone and on coarse pointers (buttons, chips, switches, tab bar, step buttons, lightbox controls); inline text links get padding or a 44px hit area on phone, and small hash chips get an invisible 44px extension.
- Keyboard: everything is reachable; slider arrow keys; player Left/Right between steps; lightbox arrows and Escape; the language and theme switches are real buttons with `aria-pressed` or a live label.
- Screen readers: charts have text alternatives (`role=img` + `aria-label`, table disclosure for the burn-down, tooltip as `aria-live`); status is word + icon + colour; decorative icons are `aria-hidden`; the language switch exposes "English" and "简体中文" names; `aria-label`s that change with language are swapped by `data-al-en` / `data-al-zh`.
- Motion and transparency: see section 6; `prefers-reduced-transparency` makes glass opaque.
- Language: `lang` on `<html>` follows the choice; individual bilingual spans carry `lang`.

## 13. Data and backend assumptions

The site is generated at build time from files in the repository; no runtime API. Requirements:

- `SPEC.md` table rows give id, priority, status, area (derive from id ranges or add an `area` column), title (first bold phrase), blocked reason, owner. **Gap:** areas and zh titles are not in SPEC.md; the build needs a small mapping file (`manager-site/areas.json` and `titles.zh.json`) or SPEC.md gains a title column in both languages.
- `plan/STATUS.md` gives "Overall", per-area "Done" lists (the `built` counts) and role tasks. **Gap:** it has no per-role "working on / last activity" field; add one (the manager already logs it) so role cards can show a task and a time.
- `plan/APPROVALS.md` gives approvals; zh text must be written by the manager.
- `reports/<id>/report.json` (kind, date, status, titles, summaries, sections[{component, data}]) drives standard reports; bespoke pages are `manager-site/src/reports/<id>/` modules.
- Tests: the build needs machine-readable results (`passed`, `failed`, `skipped`, per suite). **Gap:** `cargo test` output must be saved as JSON by the tester or the manager (`reports/<id>/tests.json`).
- Burn-down history: needs one number per day. **Gap:** nothing stores it today; the manager should append `date, open_count` to a small `plan/burndown.csv` each cycle.
- Git history: `git log` at build time (hash, date, subject). Commit subjects are shown verbatim, so they must not contain private data; zh titles for commits are optional (fall back to the English subject).
- Real-state facts used by the mockups: 63 unit tests passing, v0.3.1-rc.2 published as a pre-release, signed OTA self-update with rollback tested end to end, no web UI. If any of these changes, the report must change; do not copy the mockup text.
- Assets: every file <= 10 MB, each report <= 25 MB; filenames carry a content hash; video needs `Range` support.

---

# Part 2. Web app (R-002)

Reference implementation: `design/mockups/app/` (`assets/app.css` = every token and component class below; `assets/i18n.js` = every string in three languages; `assets/app.js` = hand-rolled renderer of all screens and states, **not** the product code). Open it with any static server or straight from disk: `cd design/mockups/app && python3 -m http.server 8090`, then http://localhost:8090/ . A **Review** tab on the right edge opens the review panel: language (English / 简体中文 / 繁體中文), appearance (Auto / Light / Dark), device frame (Window / Phone 390 / Tablet 834), and a link to every screen and state. URL switches: `?lang=en|zh-Hans|zh-Hant`, `?theme=auto|light|dark`, `?dev=auto|phone|tablet`; the route and state live in the hash, for example `#/files?state=empty`. Smoke test: `bun design/mockups/app/tools/smoke.js` renders all 59 routes in all three languages.

The product look is the report site's look, turned into an app: same blue (`--accent`, `--btn-fill`), same neutrals and hairlines, same radii and motion. What is specific to the app:

- **Content is the colour.** Chrome is neutral glass and one blue; the colour on screen comes from the family's own photos (original generated thumbnails in the mockup) and from the file-kind tiles.
- **Folders are a two-tone blue folder** (original shape, `--folder` / `--folder-2`), the signature object of the product; file kinds have a rounded tile with a glyph and a soft gradient.
- **Floating layers are glass, everything you read is solid.** Sidebar, toolbar and tab bar are translucent (`--glass`, `--glass-side`, 20 px blur). Menus, dialogs, banners and the upload panel use the denser `--glass-pop` (88 to 90 % opaque) so text contrast is computed on a near-solid surface. File content is never blurred.

## 14. Tokens added or specialised for the app

All Part 1 tokens apply unchanged (`--bg`, `--surface`, `--surface-2`, `--text`, `--text-2`, `--line`, `--accent`, `--btn-fill`, `--accent-tint`, status colours, radii, motion). New:

| Token | Light | Dark | Use / contrast |
|---|---|---|---|
| `--surface-3` | #ececf0 | #3a3a3c | skeleton highlight |
| `--glass-pop` | rgba(255,255,255,.88) | rgba(44,44,46,.90) | menus, dialogs, banners, upload panel (text >= 7:1 on the blended surface) |
| `--glass-side` | rgba(245,245,247,.78) | rgba(28,28,30,.78) | sidebar |
| `--scrim` | rgba(0,0,0,.32) | rgba(0,0,0,.55) | behind dialogs |
| `--accent-tint-2` | rgba(0,113,227,.16) | rgba(41,151,255,.24) | selected tile/row, current sidebar item (accent text on it 4.57:1 light) |
| `--folder`, `--folder-2` | #3d8ff0, #6fb1ff | #4a9bff, #7cb8ff | folder icon (graphic >= 3:1) |
| `--k-photo` `--k-video` `--k-audio` `--k-doc` `--k-other` | #f2994a #8e4fc4 #e0527a #3d8ff0 #8e8e93 | #ff9f4a #bf5af2 #ff6b8e #4a9bff #98989d | storage bar segments only; always paired with a text legend with sizes |
| `--ph` | #636366 (5.1:1 on the search field) | #a1a1a6 | placeholder text |
| `--ok` / `--warn` / `--err` + `-bg` | #1b7339 / #8a4b00 / #bf2f1a on #e0f3e6 / #fff3df / #fce8e4 | #4cd77a / #ffc971 / #ff8a7a on #203d28 / #33260f / #452323 | chips and inline messages, 4.89 to 6.2:1 |
| destructive fill | #d93a22 in both themes (white text 4.59:1; the dark `--err-fill` #ff453a would only give 3.4:1) | | `Delete Forever`, `Install Anyway`, `Empty Trash` confirm |

Rules: selected rows and tiles switch their secondary text to `--text` (secondary grey on the blue tint is only 3.99:1). One colour means one thing: blue is interactive or selected, green is healthy/finished, amber is attention or waiting, red is failure or destructive, purple only identifies video files. Status is always icon + word + colour.

## 15. Type scale (app)

Same stack as Part 1 (`--font`, `--font-hans`, `--font-hant`); the app sets `<html lang>` to `en`, `zh-Hans` or `zh-Hant`, and `:lang(zh-Hant)` selects PingFang TC / PingFang HK / Microsoft JhengHei / Noto Sans CJK TC. Sizes in px; the body default is 15 px because this is a dense desktop-class app (Apple's macOS default is 13 pt, iOS 17 pt: phone body rises to 16 px in inputs to avoid zoom).

| Role | Size / line | Weight | Tracking | Used for |
|---|---|---|---|---|
| Page title (`.h1`) | 32 / 1.12 (28 phone) | 700 | -0.025em | Settings, Trash, Storage, Admin large title; fades into the toolbar when scrolled |
| Auth title | 28 / 1.15 (26 phone) | 700 | -0.025em | sign in, register, status screens |
| State title | 24 (22 phone) | 700 | -0.02em | empty/error/updating |
| Section title (`.h2`) | 21 | 600 | -0.018em | cards, dialogs |
| Toolbar title / breadcrumb | 17 | 600 | -0.02em | breadcrumb current folder |
| Body, row title | 15 to 16 | 500 | | file names, cell labels (16) |
| Input | 17 | 400 | | text fields (48 px high) |
| Secondary | 13 to 13.5 | 400 | | dates, sizes, hints, `--text-2` |
| Caption, chip, tab label | 12.5 / 11 (tab) | 500 | | chips 12.5; phone tab labels 11 (minimum) |
| Mono | 13 | 500 to 600 | | versions, links, one-time code (24 px, 0.5em tracking) |

Numbers that change or align use `font-variant-numeric: tabular-nums` (`.num`). Chinese body line-height is 1.6, headings are not letter-spaced tighter than -0.01em. File names wrap with `overflow-wrap: anywhere` and clamp to two lines in grid, single line with ellipsis in list; the full name is the `title`.

## 16. Spacing, layout, elevation

- Base unit 4 px (8, 12, 16, 20, 24, 32, 40, 56). Page padding 24 px (16 phone). Reading pages (`.page`) are 760 px wide (980 `.wide`), centred, 28 px between groups, 20 px card padding, 20 px radius (`--r-l`), 12 px (`--r-m`) for rows and tiles, 10 px for thumbnails, 28 px for state icons tiles, pill for buttons and chips.
- Shell: sidebar 248 px (desktop) / 76 px icon rail with 10.5 px labels (tablet) / bottom tab bar 56 px + safe area (phone); toolbar 56 px (52 phone). Grid tile min 168 px (104 px phone, 2 columns under 380 px); list row 52 px (60 px phone); settings cell 56 px; share/trash row 68 px.
- Elevation: `--sh-1` ring for cards at rest; `--sh-2` for menus, banners, hover; `--sh-3` for dialogs and the upload panel. Dark theme keeps the 1 px ring and deepens shadows.
- Breakpoints (viewport, same numbers as the mockup's container queries): **phone < 700**, **tablet 700 to 1099**, **desktop >= 1100**. Layout follows available width, not device.

## 17. Motion

Same curves and durations as section 6 (`--ease`, `--ease-io`, `--ease-spring`, `--d1` .18 s, `--d2` .35 s, `--d3` .7 s). App specifics: tile hover lifts 2 px (`--d2`); selection tick fades (`--d1`); dialog scales from .94 with `--ease-spring`; phone sheets rise 40 px; menu pops from .985 (.16 s); toast and banner slide 10 px; upload progress width eases linearly (.3 s per tick); storage segments grow from the left once (`--d3`); the drop overlay's icon bobs 6 px (1.6 s alternate); skeletons shimmer 1.4 s; the indeterminate bar loops 1.5 s. Nothing animates on every list update. With `prefers-reduced-motion` all durations collapse to ~0, shimmer stops (flat `--surface-2`), the indeterminate bar becomes a static partial bar, spinners are replaced by the static icon, and progress still updates (information, not decoration). `prefers-reduced-transparency` makes every glass surface opaque `--surface`. `prefers-contrast: more` darkens `--text-2` and the hairlines.

## 18. Iconography

Same rules as section 7: original 24 px line icons, 1.75 px stroke, round caps, `currentColor`, drawn in `assets/icons.js` (about 60 glyphs: folder, file kinds, grid, list, search, upload, download, trash, share, link, sliders, user, cloud, check, x, plus, chevrons, more, restore, clock, shield, alert, wifi-off, refresh, play, pause, volume, info, lock, mail, phone, drive, sun, moon, globe, sort, move, copy, edit, expand, up, key). Sizes: 16 inside buttons, 18 in menus and action buttons, 20 in toolbars, 22 in the sidebar, 24 in the tab bar. Settings is a three-slider glyph, not a gear. No SF Symbols and no Apple glyphs. Thumbnails for images and videos are the file's own pixels (server thumbnail, see backend gaps); the mockup draws original hills-and-sun artwork (`ART()` in `icons.js`) in six palettes.

## 19. Components

Class names are those in `app.css`; states are all shown in the mockup (Review panel). Every control has a visible focus ring (3 px `--focus`, 2 px offset), a hover state on pointer devices, a pressed state (scale .97 for buttons), and min target 44 px on phone / coarse pointers (36 px on desktop).

| Component | Anatomy and variants | States |
|---|---|---|
| **App shell** `.app` | Sidebar / rail / tab bar + main (toolbar `.bar` + scrolling `.content`). Skip link first. Large title scrolls into the toolbar (`.bar.scrolled`). | normal, with banner, with offline/reconnect strip |
| **Sidebar** `.side`, `.navi` | Brand, items Files, Shared, Trash, Storage, Settings, Administration (admin only, amber count badge of pending accounts), storage mini card, account button (opens menu: language, Settings, Sign Out). Icons use accent blue. Max one level. | current (`aria-current=page`), hover, focus, badge |
| **Tab bar** `.tabs` (phone) | Files, Shared, Trash, Settings; 24 px icon + 11 px label; Settings holds Storage and Administration. Tabs navigate, never act. | current, badge |
| **Toolbar** `.bar` | Breadcrumb (`.crumbs`; phone: back chevron + current folder only), search, view segmented control, sort menu, Upload (menu: Upload Files, Upload Folder, New Folder). Phone: view toggle and a `+` icon button; search field is a row at the top of the list. | default, selection (`.bar.sel`, accent tint: count, Download, Share, Move, Rename (1 item), Move to Trash, Deselect; on phone the actions move to a floating bar above the tab bar), scrolled |
| **Breadcrumb** | Files > folder > folder; each ancestor is a link; current is `aria-current=page` and truncates; long paths collapse on phone to back + current. | link, hover, current |
| **File tile** `.tile` | 4:3 thumbnail (image/video: real pixels, video has a play badge; folder icon; other kinds: tile glyph), name (2 lines), meta (size or item count). Selection tick top-start. | rest, hover (lift, tick appears), focus, selected (accent ring 3 px + tint), drop target (blue ring), loading skeleton |
| **File row** `.lrow` | Checkbox circle, 32 px icon/thumbnail, name (+ date and size under it on phone), Date Modified, Size, more button. Sortable headers (`aria-sort`). | rest, hover, selected, focus, skeleton |
| **Menu** `.menu` | Open, Download, Share, divider, Rename, Move, divider, Move to Trash (red). Opened by the more button, right-click or long-press; also the Upload and Sort menus and the account menu. Arrow keys, Esc. | item hover = accent fill, destructive, checked (radio) |
| **Segmented control** `.seg` | View (grid/list), appearance, language, expiry, mode, channel. | selected, hover, focus, disabled |
| **Buttons** `.btn` | primary (blue fill), tint, plain, danger (tint), danger-fill, text; sizes 36 (44 phone) and `lg` 48. `.ibtn` icon button 36 (44 phone), `.abtn` toolbar action. | rest, hover, pressed, focus, disabled (.45), busy (spinner replaces the icon) |
| **Text field** `.input` | 48 px, label above, 12 px radius, inline error with icon and `role=alert`, show/hide password button. | rest, focus (2 px ring + halo), error (`aria-invalid`), disabled |
| **Switch** `.switch` | 51 x 31, spring knob. `role=switch`. | on, off, focus |
| **Cell list** `.cell-list`, `.cell` | Settings-style grouped rows: icon tile, title, subtitle, trailing control or chevron; 56 px. | rest, link, with chip |
| **Chips** `.chip` | neutral, ok, warn, err, accent; icon + word. Expiry, verified, approval, download permission, days left. | |
| **Dialog / sheet** `.dialog` | Centred 440 px dialog on tablet and desktop, bottom sheet with grabber on phone. Alert variant `.alert` (320 px, stacked full-width buttons, destructive first) only for irreversible actions: Delete Forever, Empty Trash, Stop Sharing, Install Anyway. Esc and scrim close; focus moves in and returns. | enter (spring), phase 1/2 content |
| **Toast** `.toast` | One line, optional Undo, 4.5 s, `role=status`, above the tab bar on phone. Used for Moved to Trash (with Undo), Restored, Link copied, Saved. | |
| **Banner** `.banner` | Glass pill under the toolbar: new version available (Reload, Later) / will reload after the upload. Non-blocking. | |
| **Updating overlay** `.updating` | Full-stage blur with logo, "Updating, reconnecting…", calm copy, indeterminate bar, retry countdown. Not red, not an alert. | |
| **Offline / reconnect strip** `.offline-strip` | Amber "Offline. Changes will wait." or blue "Reconnecting…" under the toolbar; content stays visible. | |
| **Upload panel** `.up` | Floating glass card bottom-end (full-width above the tab bar on phone): progress ring + "Uploading 3 items" + "2 min left to {folder}"; minimise, close; rows with file tile, name (final name if the server renamed it, with the renamed note), linear progress with percent and bytes, cancel. | uploading, waiting, paused (offline), done (check, renamed note), failed (red text + retry), cancelled, minimised (header only), all done (green ring) |
| **Drop overlay** `.drop` | Dashed blue 24 px frame over the whole content, big upload glyph, "Drop files here to upload", "They will be added to {folder}". Hovered folder tile gets the blue ring. | |
| **Empty / error state** `.state` | 96 px icon tile (blue empty, amber warning, red error, grey neutral), title, one sentence, one action. | empty folder, no results, load error, drive missing, offline, trash empty, nothing shared |
| **Skeleton** `.sk` | Same footprint as tiles or rows, shimmer. | |
| **Preview** `.pv` | Full-stage dark dialog: close, title and "n of m", share, download; image gallery with thumbnail strip, arrows and keys, swipe on touch; audio (cover, scrubber, transport, volume); video (16:9, big play, scrubber with buffered range, volume, full screen); unsupported-format state with Download. | image, audio paused/playing, video paused/playing, cannot play |
| **Storage bar** `.meter` | Segmented bar by kind + legend rows with sizes + drive row; warning banner under 10 % free. | normal, low |
| **Share row** `.srow` | Icon, name, chips (expiry, download on / view only, opened n times), Copy Link, Stop Sharing. | active, expired (red chip, copy disabled) |
| **Trash row** `.srow` | Icon, name, chip "n days left" (amber with clock icon at 1 day or less), origin folder, size, Restore, Delete Forever. | |
| **Update card** | Hero (logo, current version, state line, action) + release notes + progress steps (download / verify and install / restart); phases idle, checking, available, downloading, installing, restarting, waiting, done, failed (rolled back), busy (transfers running). | all phases |
| **Auth card** `.auth-card` | Logo, title, sub, form or status mark (72 px tile: clock = waiting, lock = rejected/closed, mail = submitted), 3-step list for "request submitted", language switch top-end. | |

## 20. Screens and states (inventory)

| Route | Screen | States designed |
|---|---|---|
| `#/signin` | Sign in | default, wrong credentials (inline, field-level `aria-invalid`), show password |
| `#/register` | Create account | default |
| `#/pending` | Request submitted (R-062, 202 with no token) | default (3-step list) |
| `#/account-pending`, `#/rejected` | `ACCOUNT_PENDING`, `ACCOUNT_REJECTED` | default |
| `#/closed`, `#/busy` | `REGISTRATION_CLOSED`, `REGISTRATION_BUSY` | default |
| `#/files[/id]` | File browser | grid, list, loading, empty folder, load error, drive missing, offline (strip, uploads paused), offline with nothing loaded, multi-select, search results, no results, row menu, new folder dialog, drag over, uploading, minimised, failed item, finished, batch download (choose, preparing), nested folder breadcrumb |
| `#/preview/id` | Preview | image gallery, audio, video, playing, cannot play |
| `#/trash` | Trash | list, empty, loading, delete-forever and empty-trash alerts, restore with undo |
| `#/shared` | Shared by you | list (active, expired), empty, create-link dialog (two phases), stop-sharing alert |
| `#/public` | Public share page (no account) | default, view only, link gone |
| `#/storage` | Storage usage | normal, low space |
| `#/settings` | Account settings | default, one-time-code dialog (send, enter), change password |
| `#/admin` | Administration / Software Update | idle, checking, available, downloading, installing, restarting, waiting, done, failed, busy, plus accounts waiting for approval |
| any | R-027 | new version banner, banner while uploading, updating overlay, live-link-lost strip |

## 21. Language, theme and device handling

`<html lang>` follows the choice (`en`, `zh-Hans`, `zh-Hant`). Default from the browser per R-005 (`zh-TW/HK/MO/Hant` Traditional; `zh-CN/SG/Hans`, bare `zh` Simplified; else English), switchable on the auth screens, in the account menu and in Settings; remembered. Theme: Automatic (follows `prefers-color-scheme`, default), Light, Dark; remembered. The three-state appearance control is a web-app necessity (browser chrome cannot be switched); the default is Automatic. Strings come from `design/glossary.md` (tables B to F) and `assets/i18n.js` is the same data in code form.

Longest-string checks done in the mockup: English `Annual family travel plan 2027, itinerary, bookings and insurance documents (final, reviewed by everyone).pdf` (clamped to 2 lines in grid, ellipsis in list), `The administrator did not approve this account. If you think this is a mistake, contact them directly.` and its Chinese equivalents at 390 px, `Pre-release builds arrive sooner and may have rough edges.`, `Waiting for the new version to come back…`, Traditional `因名稱已存在，已儲存為「…」`, Settings cell titles, the phone upload panel, and the segmented controls (language switch wraps rather than clips). Chinese is as long as or longer than English in buttons and sentences (for example 取消上傳, 清除已完成項目, 目前不開放註冊), so no control has a fixed width tied to English.

## 22. Accessibility

- Contrast values are in section 14 (tokens) and Part 1 section 2. Every text token is >= 4.5:1 on the surface it sits on; large text and graphics >= 3:1. Secondary text on the selection tint switches to `--text`.
- Focus: 3 px ring on every interactive element; `:focus-visible` only; the large-title content area takes programmatic focus from the skip link without a ring. Tab order follows reading order (skip link, sidebar, toolbar, files). Dialogs trap focus, move it to the first field or the safe button, and restore it. Esc closes dialogs, menus, then clears selection, then closes the preview.
- Keyboard: arrow keys move through tiles and rows, Enter opens, Space toggles selection, Cmd/Ctrl+A selects all, Shift/Cmd-click extends selection (desktop); preview: left/right, Space play/pause, Esc close. Suggested shortcuts for the frontend: `/` focus search, `Backspace` or `Delete` move to Trash, `U` upload.
- Screen readers: lists are `listbox` with `aria-multiselectable` and `aria-selected`; the tick is a button named "Select {file name}"; icon-only buttons have `aria-label`; progress uses `role=progressbar` with value; the upload panel is a labelled region and its summary is `role=status`; banners, toasts, the updating overlay and inline errors are live regions; storage meter has a text alternative; status chips are icon + word.
- Targets: 44 px on phone and coarse pointers (ticks and checkboxes extend their hit area 11 px beyond the glyph), 36 px on desktop.
- Reduced motion, reduced transparency, increased contrast: section 17.
- Touch: swipe left/right in the gallery, swipe-free alternatives always exist (arrow buttons on tablet/desktop, the thumbnail strip on phone).

## 23. Data and backend assumptions (app)

See `design/app-spec.md` "Backend gaps" for the actionable list. Summary: thumbnails for images and videos, storage by kind, a count of items in a folder, audio metadata and cover art, a share-link public URL (`/s/<token>`), SSE event types for transfers and trash, and approval counts for the admin badge are not in the API today.
