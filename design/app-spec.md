# Web app: implementation notes for the frontend engineer (R-002)

Read together with `design/system.md` Part 2 (tokens, type, components), `design/glossary.md` table E (strings; `design/mockups/app/assets/i18n.js` has the same keys) and the clickable mockup `design/mockups/app/index.html` (open with a static server; the **Review** tab on the right edge jumps to every screen and state, switches language, theme and phone/tablet frame). The mockup renderer (`app.js`) is throw-away; do not port it. Copy the CSS tokens and class names, and the state names below.

## 1. Build notes

- Stack per R-003: Vite + React + TypeScript. No UI kit; the components are small. Take `app.css` tokens (`:root` / `[data-theme=dark]`) as the starting stylesheet (CSS modules or one global file, your choice), replace the container queries by viewport media queries with the same numbers (phone < 700, tablet 700-1099, desktop >= 1100), or keep container queries if you want the sidebar to react to the window rather than the screen.
- Fonts: system stack only; set `<html lang>` to `en`, `zh-Hans` or `zh-Hant` (this selects the CJK face, see system.md section 15).
- Icons: copy `assets/icons.js` paths into a typed `Icon` component. Thumbnail artwork in the mockup (`ART`) is a placeholder: use the server thumbnail.
- All strings through i18n keys from glossary table E; dates, sizes and numbers through `Intl` for `en`, `zh-Hans-CN`, `zh-Hant-TW`. Sizes use decimal units (kB/MB/GB) like the mockup.
- Routes (hash in the mockup, real paths in the app): `/signin`, `/register`, `/pending` (after 202), `/files/:folderId?`, `/preview/:fileId` (opens over the folder it was launched from; Esc returns), `/shared`, `/trash`, `/storage`, `/settings`, `/admin`, `/s/:token` (public share page, no session, no shell). Status screens (`account-pending`, `rejected`, `closed`, `busy`) are not routes in the real app: render them from the error code of the response; the mockup only has routes so each one can be opened.
- Preserve state in the URL where it helps sharing and reload: folder id, search query (`?q=`), view mode is a preference (localStorage), sort is a preference.

## 2. Shell and navigation

- Desktop >= 1100: sidebar 248 px (Files, Shared, Trash, Storage, Settings, Administration for admins), storage mini card and account button at the bottom. Tablet: 76 px icon rail with small labels. Phone: 4-tab bar (Files, Shared, Trash, Settings); Storage and Administration live inside Settings. Tabs navigate, never act.
- The sidebar should be hideable (`HIG sidebars.md`: "Consider letting people hide the sidebar"): add a toggle in the toolbar (shortcut `Cmd/Ctrl+\`). The mockup does not draw it; collapse to the tablet rail is the intended hidden state on desktop.
- The account button opens a menu: language (three items by their own names), Settings, Sign Out. Language is also on the auth screens and in Settings.
- Large page titles (Settings, Trash, Storage, Admin) sit in the page; the same title fades into the toolbar after 48 px of scroll (`.bar.scrolled`). The toolbar copy is `aria-hidden`.

## 3. File browser (R-007)

State machine for the content area: `loading` (skeleton, same footprint as real items, `aria-busy`) -> `ready` | `error` (load error, drive missing, offline-with-nothing-cached). `ready` has sub-states: `empty` (folder has no items), `search` (query non-empty: results across the whole drive, header "Results for ..." + count; `no results` when none), `selecting` (>= 1 selected), `dragging` (files dragged over the window).

- **Click model.** Pointer (fine): click selects, Cmd/Ctrl/Shift-click extends, double-click or Enter opens, Space toggles, Cmd/Ctrl+A selects all, right-click opens the row menu. Touch (< 700 or coarse pointer): tap opens when nothing is selected; tapping the tick circle starts selection; with a selection, tap toggles. The tick has an 11 px invisible extension so its hit area is 44 px.
- **Selection bar.** Replaces the toolbar contents (accent tint): count, Download, Share (only for exactly one item), Move, Rename (one item), Move to Trash, Deselect. Phone: the actions float above the tab bar. Move opens a folder picker sheet (not drawn; use the same sheet/dialog components with a folder list and Move here / New Folder). Rename is inline in the name cell (grid: under the thumbnail); `409` on rename or move shows an inline error "A file with this name already exists" (add the string) and keeps editing, because rename/move do not auto-rename (backend rule).
- **Delete** is a Move to Trash with an Undo toast (4.5 s); no confirmation alert. Only irreversible actions (Delete Forever, Empty Trash, Stop Sharing, forced update install) use the alert.
- **Sorting**: Name (natural, locale-aware, folders always first), Date Modified, Size; click a list header to toggle direction; the menu lists the same. Persist.
- **Breadcrumb**: every ancestor is a link; on phone only back chevron + current folder name. Drop target: dropping files on a folder tile or a breadcrumb segment uploads into it.
- **Search**: `GET /search?q=` debounced 250 ms, field in the toolbar (phone: row above the list). Clear button, Esc clears. Results are a flat list (grid or list view) and show the parent folder name as secondary text (needs `parent_name`, see gaps).
- **Offline**: strip "Offline. Changes will wait." under the toolbar, loaded content stays, Upload is disabled, uploads in the panel show "Paused, waiting for connection" and resume automatically. If nothing is cached show the offline state with Try Again. "Live link lost" (SSE reconnecting) is a quieter blue strip with a spinner; it is not an error.

## 4. Upload (R-008)

- Entry points: Upload button menu (Upload Files / Upload Folder / New Folder), drag and drop anywhere on the file area, paste is not required.
- Drag over: full-area dashed overlay + hovered folder highlight. Remove the overlay on drag leave/drop/Esc. Implementation: count dragenter/dragleave, do not re-render the page under the cursor.
- Panel: bottom-end card (above the tab bar on phone). Header ring = overall bytes progress; title "Uploading n items", subtitle remaining time and destination folder; minimise and close (close while active = cancel all, with the alert if more than one is running). Rows: per-file progress, percent and bytes, cancel; on finish the **name returned by the server** is shown, and if it differs from the local name show the `renamed_as` note. A failed row shows the reason and a retry button; cancelled rows stay greyed until cleared. At most 2 concurrent transfers visible as "uploading", the rest "waiting" (client queue; pick a limit that suits large files).
- Folders: upload as a tree; the panel shows the folder as one row with a file count.
- Large files: progress from `XMLHttpRequest.upload.onprogress` (fetch has no upload progress). Resumable chunks are a backend decision (see gaps); until then a failed large upload restarts.
- After completion, refresh the current folder without losing scroll or selection; the new items get a 1 s soft highlight (not animated in reduced motion).
- R-027: never auto-reload while the panel has active items; show the "will reload when your upload finishes" banner.

## 5. Preview (R-009)

- Opens as a full-stage dark layer over the folder (route `/preview/:id`). Gallery order = images in the folder in the current sort. Left/right keys, arrow buttons (tablet/desktop), swipe (touch), thumbnail strip. Preload next and previous. Esc or the close button returns to the folder with the same scroll and selection.
- Image: contain, no crop; double click/tap or pinch to zoom is a P2; keep the title and "n of m".
- Audio: native `<audio>` or a custom player over `Range` requests; custom controls as drawn (scrubber is an `<input type=range>` under a drawn track for keyboard and screen reader support, step 1 s, Page keys 10 s). Show title; artist and cover only if the backend provides them (see gaps), otherwise show the generic tile.
- Video: `<video>` with the drawn controls or the native ones (native is acceptable for v1; keep the dark stage and the unsupported-format state). Buffered range is shown as a lighter track.
- Unsupported or failing media (`error` event): the "can't be played here" state with a Download button.
- Share and Download are in the top bar. Non-media files (PDF, documents) have no preview in v1: Enter on them downloads (toast "Downloading…"); PDF inline preview is a possible P2 via the browser viewer.

## 6. Batch download (R-010)

- Selection bar Download: one file -> direct download; folders or several items -> dialog "Download n items": format radio (ZIP default / tar.gz) with one-line hints, summary "Includes n folders and everything inside them · About 3.6 GB", Cancel / Download. After pressing Download, show the "Preparing" phase of the dialog (indeterminate bar) and start the browser download through a normal navigation or hidden link to the archive endpoint (the server streams while packing, so no waiting screen is needed; the dialog can close as soon as the browser takes over). The dialog's second phase in the mockup exists to show copy for the slow case; the final UX may skip it if the response starts immediately.
- Size shown is an estimate (sum of sizes the browser knows); folders need a size from the API.
- Download progress is shown by the browser. R-027: block auto-reload while a download is running if the app can know (keep a counter for archive downloads it started).

## 7. Trash, sharing, storage (R-011)

- **Trash**: rows with icon, name, origin folder, size and the days-left chip (`expires_at` -> whole days left, 0 = "Deletes today"; amber with clock at <= 1 day). Restore (button) with an Undo toast "Restored to {folder}"; if the origin was deleted the backend restores to the root: toast says so. Delete Forever and Empty Trash use the alert. Empty trash state invites nothing (nothing to do). Bulk actions: not drawn; `Empty Trash` is enough in v1.
- **Share**: Share on exactly one item opens the dialog: expiry (1 day / 7 days / 30 days / never) and Allow downloads (switch); Create Link -> second phase with the link, Copy Link and the reminder "Anyone with the link can open it". The list screen shows expiry, view-only/downloads, open count (`access_count`) and Copy Link / Stop Sharing; expired shares are red-chipped with Copy disabled. Public page `/s/:token` (no shell): preview thumbnail, name, "Shared by {name}", size, Download (hidden when `can_download` is false, with a "View only" chip), the expiry; link gone state when expired or revoked.
- **Storage**: big used number, segmented bar by kind with a legend that carries sizes, drive card with connected state; under 10 % free shows the amber warning with a link to the list view sorted by size. Live updates through SSE: the numbers tween (reduced motion: jump).
- **SSE**: refresh the visible folder, the trash, the shares and the storage numbers on matching events; if the connection drops show the quiet reconnect strip, retry with backoff, and after reconnect compare `GET /health` version (R-027).

## 8. Account, auth and approval (R-006, R-012, R-062)

- **Sign in** (`POST /auth/login`): identifier (email or phone), password with show/hide. `401` -> field-level inline error "The email, phone number or password is not right." on both fields (no hint which one; same message), focus returns to the identifier. `403 ACCOUNT_PENDING` and `403 ACCOUNT_REJECTED` -> the full-screen status screens (no session is created). Never display the administrator's contact details.
- **Register** (`POST /auth/register`): display name, email or phone, password. Outcomes: `201` (auto-approved or first account) -> signed in -> Files; `202 pending_approval` -> the "Request submitted" screen with the three steps (Request sent / Administrator reviews / You can sign in); `REGISTRATION_CLOSED` -> closed screen; `REGISTRATION_BUSY` -> busy screen with Back to Sign In (also offer Try Again after a while). All four status screens have only one action, Back to Sign In.
- **Language switch** is visible on every auth screen (top-end) and remembered; first visit follows the browser (R-005).
- **Settings**: Display name (inline field + Save), Ways to sign in (email, phone: verified chip or amber "Not verified" + Verify; Change / Remove; keep at least one), Change Password, Language, Appearance, Storage and Administration cells, Sign Out. Identifier changes go through the one-time-code dialog: `POST /me/identifier/challenge` -> six-digit field (`autocomplete=one-time-code`, `inputmode=numeric`) -> `PUT /me/identifier` / verify; resend with a countdown. Identifiers with `managed_by` are read-only with the "Managed by the administrator" caption (key in table E).
- Session: `GET /auth/me` on load; `401` anywhere -> sign-in screen with a toast "Please sign in again" (add string), keep the intended route.

## 9. Admin and update (R-013, R-020, R-027)

- Administration appears only for the admin. The update card phases map to `GET /admin/update` `phase`: idle, checking, downloading (with percent if the API provides it), installing, restarting, failed (+ last error). `409` on install (transfers active) -> the "N transfers are running" dialog with Cancel / Install Anyway (`force`).
- One product version (no UI version). After Install: show progress steps; when the SSE stream closes or `/health` fails, show the **Updating, reconnecting** overlay (calm, not an error) and retry with backoff (1 s, 2 s, 4 s ... capped at 10 s, counter visible); when `/health` returns the new `version`, reload and show a "Updated to v..." toast. If it never returns within a few minutes, keep the overlay and add a "Try again" button; do not show a stack trace.
- **New version available** (the open tab's bundle version differs from `/health`): non-blocking glass banner "A new version is available / Reload to switch to {v}. Your uploads are not interrupted." with Reload and Later. Auto-reload only when the user is idle (no input for ~60 s) and no upload/archive download is active; while one is running show the second copy ("Will reload when your upload finishes"). A failed dynamic import triggers the same reload path.
- Settings: update mode (Manual/Automatic), window (the select lists 2-hour windows), channel (Stable / Pre-release with the caution text). Accounts waiting for approval: list with Approve / Reject (`POST /admin/users/approve|reject`); count on the sidebar badge and the phone Settings tab.

## 10. Accessibility checklist for implementation

Skip link; focus ring everywhere; `listbox`/`option` semantics with `aria-multiselectable`; tick button named "Select {file}"; menu keyboard model (arrows, Esc, Home/End, typeahead); dialogs `role=dialog|alertdialog`, labelled, focus trap and return; toasts, banners and inline errors as live regions; progress bars with `role=progressbar`; `prefers-reduced-motion` (no shimmer, no bob, no sweeps, progress still updates), `prefers-reduced-transparency` (opaque surfaces), `prefers-contrast: more`; 44 px targets on touch; `lang` on the html element and on the language buttons; no color-only meaning (chips carry icon + word, charts carry legends with numbers); text up to 200 % zoom without horizontal scroll (the layout reflows; tab labels may wrap to two lines).

## 11. Backend gaps (data the design assumes that `/docs` does not provide)

1. **Thumbnails** for images and videos (`GET /files/:id/thumbnail?size=`) with cache headers; and a `kind` or `mime_type` field in the list response (the UI picks icons from it).
2. **Folder item count and size** in `GET /files` (`child_count`, `size_total`), and **parent name/path** in search results (`parent_id` and `parent_name`).
3. **Storage usage by kind** (photos, videos, audio, documents, other, trash) and drive capacity/free: `GET /storage` or extend `GET /drives`.
4. **Batch download size estimate** and a single endpoint that accepts several ids and a format (`zip` | `tar.gz`); today only `file.download_archive` is mentioned in audit logs. Response must stream (the design relies on it) and set `Content-Disposition` with a localized-safe filename.
5. **Audio metadata** (title, artist, cover art) for the player; fallback is the file name.
6. **Public share URL**: the share object should include the absolute path (`/s/<token>`), `can_download`, `expires_at`, `access_count`, and the public page needs a JSON endpoint for name, size, `shared_by`, mime type and (when allowed) a Range-capable download and a thumbnail.
7. **Resumable / chunked uploads** for very large files and an upload size limit value for the UI to show.
8. **SSE event vocabulary** documented for the UI: file created/renamed/moved/deleted, trash changed, share changed, storage changed, drive attached/detached, update phase changes, and `version` in a `hello` event on connect (R-027 compares it with `/health`).
9. **Update phase detail**: percent while downloading (`progress`), `installed_at`, release notes URL and the version to be installed, as in R-020; `GET /admin/update` should also report running transfer count so the UI can warn before `409`.
10. **Account approvals count** for the admin badge (`GET /admin/users?status=pending` is enough; a lightweight count is nicer).
11. **Copy** (duplicate) is not offered in the UI because the API has no copy endpoint; add it if wanted (the glossary already has Copy).
12. **Rename/move conflict error code** (`409` today): add a machine-readable `code` (for example `NAME_TAKEN`) so the UI can show the localized inline message.
13. **Recent** (a "Recent" sidebar item) needs a "modified since" query; not designed. Add as a later requirement if the owner wants it.

## 12. Open questions for the owner

1. Show file extensions? The mockup shows full names (the server's `name(1).ext` rule makes extensions meaningful); Apple's guidance is to hide them by default with a switch. Keep visible?
2. Is the appearance control (Automatic / Light / Dark) wanted in Settings, or follow the system only? The mockup has it.
3. Do family members see each other's shares (no: Shared means "shared by you")? Is "shared with me" needed?
4. Which languages should the public share page offer, and does it need a language switch? The mockup has the same switcher as sign-in.
5. Phone number format and country code: the mockup shows a free-text field; a country picker is not designed.
