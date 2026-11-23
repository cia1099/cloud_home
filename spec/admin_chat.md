# Cloud Home — Account approval and admin chat integration

Status: **requirements / design** (not implemented). Tracked in `SPEC.md` as R-060 … R-068.
Code comments follow the repo convention (Chinese); this document is English so the owner can review it.

## 1. Why

Cloud Home is a *private* drive. Today anyone who can reach `POST /auth/register` gets an active account and a
token immediately. The owner wants every new account to be **approved by an administrator** before it can touch
the home drive, and wants to do that from a chat app on their phone instead of calling the private `/admin/*`
endpoints (shared password + local network only) by hand.

Goals
- No account can sign in until an admin approves it (fail closed, including when the chat app is down).
- The admin approves / rejects with one tap in a chat app, from anywhere.
- The device never opens an inbound port for this (it sits behind a home router).
- The chat provider is replaceable: Telegram first, others later.
- Later phases expose the existing private admin operations and system alerts in the same chat.

Non-goals
- Chat as a channel for end users (applicants are not in the chat; they see a "waiting for approval" state in the web app).
- Roles/permissions: every paired admin is equally powerful, as the shared admin password is today.
- Webhook-based providers (LINE etc.) in the first version — they need a public HTTPS endpoint (see R-068).

## 2. Account approval model

States (new): `pending` → `approved` | `rejected`. Separate from the existing *soft-delete* (`deleted_at`).

Schema (additive migration; all existing rows become `approved`):
- `users.approval_status TEXT NOT NULL DEFAULT 'approved'` (`pending|approved|rejected`)
- `users.approval_decided_at TEXT`, `users.approval_decided_by TEXT` (`bootstrap` | `admin-endpoint` | `chat:<provider>:<sender-id>`), `users.approval_note TEXT`
- `users.registered_ip TEXT` (shown to the admin; personal data, never leaves the device except in the admin's chat)

**Fail-closed representation:** `pending` and `rejected` accounts have `is_active = 0`. Every existing check
(`find_for_login`, `AuthUser` extractor) already refuses `is_active = 0`, so a code path that forgets the new column
still denies access. *Approve* sets `is_active = 1`. Restore/soft-delete logic must never reactivate a
non-`approved` account (restore requires `deleted_at`, which pending accounts do not have — add a test).

Registration mode `REGISTRATION_MODE` (default `approval`):
| mode | behaviour of `POST /auth/register` |
|---|---|
| `open` | as today: active account, token + cookie, `201` |
| `approval` | account created `pending`; **no token, no cookie**; `202 {"status":"pending_approval"}` |
| `closed` | `403 REGISTRATION_CLOSED` |

**Bootstrap rule (owner decision):** in `approval` mode the first account is auto-approved
(`approved_by = bootstrap`) so the owner can get in — but **only if** the `users` table is completely empty
(soft-deleted rows count) **and** the caller's IP is loopback/private (same test as `AdminGuard`). Decided inside one
transaction so two simultaneous first registrations cannot both win. A remote first registrant is `pending`.

Login behaviour: wrong credentials → `401` exactly as today (no state leaks). **Correct** password on a
`pending` account → `403 ACCOUNT_PENDING`; on a `rejected` account → `403 ACCOUNT_REJECTED`.

Abuse limits (all configurable, sane defaults): `REGISTER_RATE_PER_IP_PER_HOUR=5`, `PENDING_MAX=20`
(beyond it `429 REGISTRATION_BUSY`), an identifier that was rejected cannot register again for `REJECT_COOLDOWN_DAYS=7`
(its row stays, which also keeps it from being reused), and pending accounts older than `PENDING_EXPIRE_DAYS=14`
are auto-rejected by a background task (audited).

## 3. Private admin endpoints (single source of truth)

Same guard as today (`AdminGuard`: feature switch → local network → `X-Admin-Password`). The chat bot calls the
**same service functions**, not HTTP, so authority and audit are identical.
- `GET /admin/users?status=pending|approved|rejected|all` (extends the existing list; includes `approval_status`, `registered_ip`)
- `POST /admin/users/approve` `{identifier, note?}` and `POST /admin/users/reject` `{identifier, reason?}` — idempotent; deciding an already-decided account returns `409` with who decided.
- `POST /admin/chat/pairing` → `{code, expires_at}` (one-time pairing code, see §5)
- `GET /admin/chat` → `{provider, connected, admins:[{id, label, paired_at}]}`; `DELETE /admin/chat/admins/{id}` unpairs.

## 4. Chat integration architecture

Provider-neutral core in `src/chat/`:
- `trait ChatProvider`: `send(to, Message) -> MessageRef`, `edit(MessageRef, Message)`, `next_updates() -> Vec<Update>`.
  `Message` = localized text + optional buttons. `Update` = `Command{sender, chat, name, args}` | `ButtonPress{sender, chat, message, token}`.
  Senders are the provider's **immutable numeric id**, never a username.
- `TelegramProvider`: Bot API over HTTPS with **long polling** (outbound only; no webhook, no inbound port). Uses the
  existing `reqwest` dependency; no large bot framework unless justified.
- `FakeProvider` (in-memory) for tests — the whole flow must be testable with no network.
- `outbox` table: notifications are queued and delivered with exponential backoff; the chat being down loses nothing,
  and the pending list is always available through the admin endpoints.

Configuration: `CHAT_PROVIDER=none|telegram` (default `none`), `CHAT_BOT_TOKEN`, `CHAT_LANGUAGE=en|zh-Hans|zh-Hant`
(default `en`), `CHAT_MASK_IDENTIFIERS=true`. With `CHAT_PROVIDER=none` nothing changes except that approval is done with the admin endpoints.

### Flow (Phase B)
1. A registration becomes `pending` → each paired admin gets a message: display name, **masked** identifier
   (`a***@gmail.com`, `+886 9** *** 123`), time, source IP, and buttons **Approve** / **Reject**.
2. Tap → `ButtonPress`. The server verifies: sender is a paired admin; the chat is a private chat with the bot; the
   button token exists, is unexpired (24 h), unused, and bound to (that user, that action); the account is still
   `pending`. Then it executes through the service layer, writes the audit event, and **edits the original message**
   ("✅ Approved by <label> at <time>", buttons removed). A second tap, or the other admin tapping later, gets
   "already decided by …".
3. Button payloads are **opaque random tokens** mapped server-side; they contain no user id, identifier or action.
4. Bursts are coalesced: more than 5 registrations in 10 minutes → one summary message pointing at `/pending`.
5. Once a day, if anything has been pending > 24 h, one reminder digest.
6. Commands: `/start`, `/help`, `/pair <code>`, `/pending`, `/whoami`.

### Pairing (how an admin becomes an admin)
The operator calls `POST /admin/chat/pairing` (password + local network) → 8-character one-time code, valid 10 min,
stored hashed, single use. The admin sends `/pair <code>` to the bot → their provider id is stored in
`admin_chat_identities`. No numeric ids to look up by hand, and no way in without the private endpoint. Everyone else
who writes to the bot is **silently ignored** (audited, rate-limited) — the bot never reveals it exists to strangers.

## 5. Security requirements

- Only paired admins can do anything; authority is checked on **every** update, not at connection time.
- The bot token lives only in env/`.env` or `~/.config/cloud_home/`; redacted in `Config`'s `Debug`; never logged,
  never in the repo, reports, or the public report site. Document token rotation.
- Passwords, tokens, file names and file contents are never sent to the chat. Identifiers are masked by default
  (the provider is a third party; document exactly what is sent in `spec/admin_chat.md` and README).
- Replay, forgery and double-spend: tokens single-use, expiring, bound to user+action; decisions are idempotent and
  transactional (two admins tapping at once → exactly one wins).
- Destructive operations (Phase C: delete account, purge trash) need an explicit confirmation step (a short code or
  typing the account name); `purge` is irreversible and says so.
- Outage never grants access; `pending` stays `pending` until an admin acts, however the system fails.
- Audit events (existing `crate::audit!` conventions): `user.register` (result `pending|bootstrap|open`),
  `user.approve`, `user.reject`, `user.expire`, `chat.pair`, `chat.unpair`, `chat.command` (authorized/unauthorized,
  with provider + sender id, never message bodies), `chat.delivery` (failure after retries).

## 6. Phases

| Phase | Scope | SPEC ids |
|---|---|---|
| A — approval without chat | states, registration modes, bootstrap, login errors, admin approve/reject/list, limits, expiry, UI pending states, tests, docs | R-060, R-061, R-062, R-066 |
| B — chat core | provider trait, Telegram long polling, pairing, notification + buttons, callbacks, outbox, reminders, fake provider, tests | R-063, R-066 |
| C — admin operations | `/users`, `/delete`, `/restore`, `/purge`, `/status` with confirmations | R-064 |
| D — alerts | update available (tap to install, uses R-020), drive unplugged/plugged, low disk, repeated admin-auth failures | R-065 |
| E — more providers | LINE etc. (needs a public webhook decision) | R-068 |

Phase A ships first and is useful on its own (approval via the private endpoints).

## 7. Compatibility

- **Breaking API change (owner-approved 2026-11-23):** in `approval` mode `POST /auth/register` returns `202` with no
  token. Update `spec/backend.md`, `README.md`, `.env.example`, the OpenAPI annotations, CHANGELOG (`Changed`, marked
  Breaking) and the web app (R-006/R-062).
- Existing users are migrated to `approved`; existing sessions keep working.
- Dev databases: the migration is additive (new numbered file), no rewrite of existing migrations.

## 8. Acceptance (testable)

1. Fresh DB, local caller: first registration is approved and can log in; the second is `pending`, gets `202` and no token/cookie.
2. A remote caller registering first on an empty DB is `pending` (bootstrap needs local/private IP).
3. `pending`/`rejected` with the correct password → `403 ACCOUNT_PENDING` / `ACCOUNT_REJECTED`; wrong password → `401`; the account cannot obtain or use any token.
4. Approve via the admin endpoint → login works; approve is idempotent; reject then approve is allowed and audited.
5. Restore of a soft-deleted account never reactivates a `pending`/`rejected` account.
6. `closed` mode → `403`; rate limit and `PENDING_MAX` → `429`; cooldown after rejection; expiry task auto-rejects.
7. With `FakeProvider`: registration notifies all paired admins with masked identifiers; a button press by a paired admin approves and edits the message; by an unpaired sender is ignored and audited; replayed, expired and wrong-bound tokens are refused; two simultaneous presses → one decision.
8. Pairing: a code works once, expires after 10 min, wrong codes are rate-limited; `DELETE /admin/chat/admins/{id}` immediately revokes.
9. Chat outage: registrations still succeed as `pending`, notifications are delivered later by the outbox, nothing is approved implicitly.
10. No secret or unmasked identifier appears in logs; `Config` `Debug` redacts the bot token.
11. Live check (manual, needs a real bot token): the owner receives the message on their phone and the Approve button works end to end.
12. Web app: register shows "waiting for approval", login shows the pending/rejected message — in English, 简体中文 and 繁體中文.

## 9. Open items for the owner
- Create a Telegram bot with @BotFather and provide its token (needed only for the live check, R-063 acceptance 11).
- Chat language for the bot's messages (default English).
- Single admin or several? The design supports several; the default is whoever pairs.
- Defaults for the abuse limits above (5 registrations/hour/IP, 20 pending, 7-day cooldown, 14-day expiry).
