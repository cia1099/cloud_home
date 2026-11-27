# apple-design review: web app mockups (R-002)

Method: the `apple-design` skill, five lenses, references opened: `alerts.md`, `sidebars.md`, `drag-and-drop.md`, `undo-and-redo.md`, `file-management.md`, `loading.md`; accessibility numbers computed from the token hex values (WCAG 2.x). Platform: web app (principles and foundations apply; Apple platform conventions used as design principles). Thesis: *the family's files are the colour; the interface is calm glass and one blue.* What it will be remembered by: the two-tone blue folder, original landscape thumbnails and the floating upload panel whose ring turns green when everything is safe.

**Limit of this review:** the session had no browser, so the mockup was verified by a stub-DOM smoke test (`bun design/mockups/app/tools/smoke.js`: 59 routes x 3 languages plus click paths, no exceptions, no `undefined`, no missing string keys) and by reading the CSS, not by looking at rendered screenshots. Pixel-level checks (overlaps, clipping at 390 px, CJK wrapping) must be done by the owner or the tester with the Review panel before the frontend relies on them.

## Rating: Good (no Critical open; High items below are design-to-build decisions)

## Found and fixed during the review

| Lens | Finding | Numbers | Fix |
|---|---|---|---|
| Accessibility (Critical) | Secondary text on the selected tile/row tint | `--text-2` #6e6e73 on the 16 % blue tint is 3.99:1 (< 4.5) | selected items switch secondary text to `--text` (13:1) |
| Accessibility (Critical) | White text on the dark theme's destructive red | #ff453a with white 3.4:1 | destructive fill is #d93a22 in both themes (4.59:1) |
| Accessibility (High) | Search placeholder | #6e6e73 on the grey field about 4.2:1 | `--ph` #636366 (5.1:1) light, #a1a1a6 dark |
| Accessibility (High) | Touch targets 36 px on phone | `layout.md`/accessibility: 44 pt default | buttons, icon buttons, menu rows 44 px on phone; ticks and checkboxes get an 11 px invisible hit extension |
| Accessibility (High) | Text over translucent menus, dialogs, upload panel | 72 % glass over variable content | new denser `--glass-pop` (88 to 90 %); only bars use the thin glass; reduced-transparency makes all opaque |
| Platform (High) | Title duplicated (toolbar + page heading) | | large title in the page, fades into the toolbar on scroll; toolbar copy `aria-hidden` |
| Interaction (Medium) | Alerts for undoable actions would break `alerts.md` ("Avoid displaying alerts for common, undoable actions") | | Move to Trash and Restore use an Undo toast; alerts only for Delete Forever, Empty Trash, Stop Sharing, forced install; verb button titles, destructive first, Cancel always present |
| Interaction (Medium) | Drag and drop (`drag-and-drop.md`: show whether a destination accepts, offer alternatives, feedback while transferring) | | overlay + folder target ring, Upload menu as alternative, upload panel as transfer feedback |
| Craft (Medium) | Dialog z-order under the preview layer; grid track could grow with content; duplicate drag overlay on re-render | | z-index scale fixed, `grid-template-rows: minmax(0,1fr)`, overlay inserted without re-rendering |

## Improvements still open

- **High: sidebar cannot be hidden on desktop** (`sidebars.md`: "Consider letting people hide the sidebar"). Specified in `app-spec.md` section 2; not drawn. Tablet rail is the collapsed state.
- **Medium: file extensions are always shown**; `file-management.md` says hide by default with an option. Kept because the server's `name(1).ext` rule makes them meaningful; asked as an owner question.
- **Medium: Move needs a folder-picker sheet and Rename needs an inline editor**; both use existing components but are not drawn as screens.
- **Medium: phone "Settings" tab holds Storage and Administration**, so an admin reaches pending approvals in two taps; the badge on the tab compensates.
- **Low: long-press for the row menu and gallery pinch-zoom** are specified but not demonstrated.
- **Low: the Review panel** (mockup chrome, not product) sits on the right edge and may cover a scrollbar in a narrow window; use the phone/tablet frames to avoid it.

## What works (keep)

- One accent and one meaning per colour; status is icon + word + colour everywhere (chips, upload rows, update phases, share expiry).
- Calm failure language: *Updating, reconnecting…* is blue and reassuring, the offline strip says what will happen to changes, the failed update says the old version is running.
- Approval states are four distinct, plain screens with one action each and no contact details (R-062).
- Strings were written for the three languages separately (Taiwan wording: 檔案, 資料夾, 垃圾桶, 儲存, 復原, 還原, 核准, 軟體, 網路); layouts have no fixed widths tied to English; the longest names clamp or wrap.
- Layout follows width, not device: rail and tab bar are the same destinations as the sidebar.
