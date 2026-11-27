# Report site: apple-design review (R-001, R-004)

**Scope.** `design/mockups/report-site/` at 390 and 1280px, light and dark, en and 简体中文, in headless Chromium.
Checks: overflow, target sizes, console errors, link crawl, lightbox, slider keys, language and theme switches, reduced-motion styles.
Apple references: `accessibility.md` (contrast, 44x44 targets, Reduce Motion), `tab-bars.md`, `charting-data.md`, plus the always-load set.

**Summary.** Rating after fixes: **Good**. Thesis: tell the owner the truth about a young project, quickly, as a calm Apple-style product page. Remembered by: the animated architecture and OTA diagrams (failure paths written out, the unbuilt web app drawn dashed) and the honest real state.

| # | Issue | Severity | Fix | Status |
|---|---|---|---|---|
| 1 | Language switch hid the inactive language button; no way back to English | Critical | Switch buttons exempt from bilingual-span CSS, `aria-pressed`; verified zh/en/zh | Fixed |
| 2 | Sample numbers looked like real status | High | Real state is default; sample only via `?state=sample` under a "Sample" strip | Fixed |
| 3 | Burn-down labels shrank to ~7px on phone | High | Chart drawn at container pixel width, redrawn on resize | Fixed |
| 4 | 390px Overview scrolled horizontally | High | Phone grids use `minmax(0,1fr)`; all pages 390/390 | Fixed |
| 5 | Dark primary button white on #0a84ff = 3.65:1 | High | `--btn-fill` #0071e3 = 4.70:1 | Fixed |
| 6 | Targets under 44pt on phone | High | 44px on phone and coarse pointers; hash chips get a 44px hit area | Fixed (a few footer links ~41px, not re-measured) |
| 7 | Dark todo mark #5a5a5e on #1c1c1e = 2.48:1 | Medium | #6e6e73 (~3.3:1) | Fixed (dark) / accepted (light) |
| 8 | OTA diagram unreadable on phone; failure path only a dashed strip | High | 8 numbered steps, 4x2 desktop / vertical phone, failure outcomes written out | Fixed |
| 9 | Reduced motion left loops running | High | Static final state, no autoplay | Fixed |
| 10 | Lightbox: no scroll lock/inert, no touch, 36px controls | High | Inert page, focus trap and return, swipe, 44px controls | Fixed |
| 11 | Slider focus ring wrong, handle covered labels | Medium | Native range input, arrow keys verified | Fixed |
| 12 | Placeholder art imitated a file-browser UI | Medium | Abstract art with "Sample image" tag | Fixed |
| 13 | `…/index.html` links; no `/latest/` | Medium | Folder URLs ending in `/`; `/latest/` added | Fixed |
| 14 | Placeholder/lightbox/player names English-only in zh | Medium | `data-ph-*` / `data-al-*` swap on language change | Fixed (partly, see 20) |
| 15 | Requirement totals wrapped awkwardly on phone | Medium | Grid of chip + count pairs | Fixed |
| 16 | Class collision turned diagram descriptions green | Low | Renamed class | Fixed |
| 17 | Simplified Chinese line breaking | Medium | `line-break:strict`, `overflow-wrap`, `minmax(0,1fr)` | Fixed |
| 18 | `/product/` empty state and populated variant | Medium | Honest empty state plus labelled review toggle | Fixed |
| 19 | Phone "More" tab hides Timeline and Product | Medium | Kept: six destinations do not fit five tabs | Open (trade-off) |
| 20 | Some `aria-label`s English-only in zh | Low | Frontend localizes all aria-labels from one string table | Open |
| 21 | Font sizes in px ignore the browser default size | Medium | Frontend uses `rem` | Open |
| 22 | Milestone template ignores light mode | Low | Intentional, secondary text 7.64:1 | Accepted |
| 23 | Not tested: real Safari/iOS, VoiceOver, high-contrast, 200% zoom | Medium | Graceful fallbacks; needs a pass on a real iPhone | Open (limit) |

**Craft notes.** Strong point of view: honesty is the identity (dashed "not built yet" module, written failure outcomes, empty product page, the 0% ring). Template risk: the milestone hero (gradient numerals, glow) is the genre cliché; acceptable only on bespoke milestone pages.

**Keep.** Folder URLs and pre-filled-issue buttons with no backend; pending decisions always at the top; per-component data shapes in `system.md`; text alternatives on every chart; reduced motion as a designed state; sample content impossible to mistake for real.
