# Bugs found by the tester

## B-001 (P1, security/correctness) A chat button can overturn a concurrent decision; two decisions on one account

**Status: fixed** on `agent/B-001-decide-race`. `approval_service::decide` takes `overturn_rejection: bool`
(admin endpoint `true`, chat button `false`); the approve UPDATE only matches `rejected` when it is true, so a
losing chat Approve gets 0 rows -> `Conflict` -> "Already decided". Unit test
`chat_style_approve_cannot_overturn_a_rejection` added. The 200-iteration loop is recorded in the report.

Found while diagnosing the flaky `chat::flow_tests::simultaneous_presses_produce_exactly_one_decision`
(fails about 1 run in 4, `assertion left == right: left 0, right 1` on `losers.len()`).
The test is correct; the product code violates spec/admin_chat.md §8 item 7 ("two simultaneous presses -> one decision")
and §5 ("two admins tapping at once -> exactly one wins").

**Where:** `src/chat/dispatch.rs`, `on_button` (the block that starts with `if user.approval_status == APPROVAL_PENDING`),
together with `src/services/approval_service.rs`, `decide`, `Decision::Approve` branch.

**Cause (verified by measurement, below):** `on_button` reads the account (`user_service::get`), checks
`approval_status == pending` on that snapshot, then calls `decide(..)`. Between the read and the UPDATE another press
can commit. For a **reject** that is harmless (its UPDATE is `WHERE approval_status = 'pending'`). For an **approve** it is not:
the approve UPDATE is `WHERE approval_status IN ('pending', 'rejected')`, because the admin endpoint is allowed to
overturn a rejection ("reject then approve is allowed", spec section 8 item 4). So when admin 2's Reject commits first
and admin 1's Approve already passed the stale `pending` check, the Approve succeeds (`rows_affected == 1`, `changed = true`):
- both presses "win": two audit events (`user.reject` then `user.approve`) for one account, the rejection is silently reversed
  and the account ends **approved**;
- nobody gets the "Already decided ..." reply (the test's `losers == 0`);
- the message edits are refreshed from the final state, so the rejecting admin sees "Approved by <other admin>".
A rejection made in chat can therefore be reversed by a concurrent chat tap that the rejecting admin never intended
to compete with. Overturning is meant to be a deliberate action through the admin endpoint, not a race outcome of a button.

**Evidence:** temporary instrumented copy of the test (not committed), 40 iterations, two admins pressing
Approve and Reject through `tokio::join!`:

    final=approved by=chat:telegram:1 already_decided_msgs=1 audit_approve=1 audit_reject=0   x31
    final=approved by=chat:telegram:1 already_decided_msgs=0 audit_approve=1 audit_reject=1   x9   <- the bug

(An earlier 60-iteration run gave 12 of 60 with `already_decided_msgs=0`; in every case the final state was `approved`.)

**Not covered by a committed regression test** because the window is a few microseconds and cannot be forced from
outside the crate; the committed `simultaneous_decisions_from_chat_and_http_produce_exactly_one`
(`tests/chat_fake.rs`) uses same-direction decisions (approve vs approve), which is deterministic and passes.
`chat::flow_tests::simultaneous_presses_produce_exactly_one_decision` is the regression test once fixed.

**Suggested fix (no parallel versions, per CLAUDE.md):** give `approval_service::decide` an explicit parameter for whether a
rejection may be overturned (admin endpoint: yes; chat button: no), and make the approve UPDATE's status list depend on it:
chat passes `false`, so it only matches `approval_status = 'pending'`; 0 rows then falls through to the existing
`AppError::Conflict` arm in `on_button`, which already maps to "Already decided ..." and sets `won = false`.
Equivalent alternative: make `on_button` do the whole transition in one conditional UPDATE keyed on `pending`
and drop the read-then-check. Expiry (`expire_pending`) already only touches `pending`.
After the fix run the flaky test in a loop (for example 200 iterations) to confirm it never fails.

## B-002 (P2, data safety / disk leak) An upload interrupted midway leaves its bytes on disk with no database row

**Status: fixed** (branch `agent/B-002-upload-orphan`: drop guard deletes the partial file unless the row is inserted). Found by `tests/api_http.rs::upload_interrupted_midway_leaves_no_orphan_file_on_disk`
(the only test in the suite that fails today; it passes once the bug is fixed).

**Repro (against a running server, any account, token in `$T`):**
1. Open a TCP connection to the server and send
   `POST /api/v1/files/upload` with `Authorization: Bearer $T`, `Content-Type: multipart/form-data; boundary=B`,
   `Content-Length: 1000000+...`, then the multipart header of a `file` part named `half.bin` and 100 000 bytes of body.
2. Close the connection without sending the rest (a browser tab closed or a dropped Wi-Fi link does the same).
3. `GET /files` shows nothing and `GET /usage` shows `used_bytes: 0`, but
   `find <drive>/cloud_home_data/users/<user_id> -type f` lists one file (the partial upload, named by its random id).

**Expected:** a failed upload leaves nothing behind (the comment in `handlers/files.rs` says so: "失败时清理已写入的物理文件，避免孤儿").
**Actual:** the partial file stays forever. It is not in `files`, so it is invisible to usage numbers, the trash cleaner and
account purge; it only costs disk space (repeated failed uploads of large files fill the drive silently).

**Where (read, not speculated about further):** `src/handlers/files.rs` `upload`. The `remove_file(&physical_path)` cleanup
only runs when `finalize_upload` returns `Err`. Errors raised earlier, inside the `while let Some(..) = multipart.next_field()`
loop (`field.chunk()` -> `BadRequest("文件流读取失败")`, `out.write_all(..)?`), return with `?` before that cleanup, and a client
disconnect that drops the handler future skips it as well.

## Notes (not bugs)

- Removing the last identifier (`DELETE /me/identifier` for the only email/phone) is rejected, but only after the one-time code
  was verified and consumed, so the user must request a new code (it also counts toward the 3-per-hour limit). Harmless, listed
  because `tests/api_http.rs` has to request a fresh code after that attempt.
- `GET /usage/breakdown` returns every account's username and byte totals to any logged-in user. That is what the spec says
  ("任意已登录用户可见"); the isolation test asserts it contains no file names. Flagging it in case the owner considers per-account
  byte totals private.

- `cloud_home` is a binary-only crate and `FakeProvider` exists only under `cfg(test)`, so the integration tests in
  `tests/` cannot link against it. `tests/chat_fake.rs` therefore re-compiles the same sources with `#[path]` (see the
  header comment there). Splitting `src/` into `lib.rs` + a thin `main.rs` would remove that boilerplate
  and also stop the mirrored unit tests from running twice.
- `cargo test` reports `lifecycle.rs: method wait_idle is never used` (a pre-existing idle-code warning, R-070).

## B-004 (P2, API contract) Malformed or ill-typed JSON bodies get a `text/plain` axum rejection, not the JSON error envelope

**Status: fixed** (branch `agent/B-004-json-envelope`: one `ApiJson` extractor in `src/error.rs` replaces `axum::Json` for request bodies; codes `INVALID_JSON` 400, `INVALID_BODY` 422, `UNSUPPORTED_MEDIA_TYPE` 415, `PAYLOAD_TOO_LARGE` 413; the probe test now requires the envelope). Originally: Found by the R-030 second round (`malformed_json_bodies_are_client_errors_and_change_nothing`,
`tests/api_http.rs`; it asserts only "4xx, nothing changed", so the suite stays green until this is fixed;
then tighten it to require the envelope).

`spec/backend.md` line 298 says every error is `{ "error": { "code", "message" } }`. Every JSON endpoint uses the stock
axum `Json` extractor, whose rejection bypasses `AppError` (no `JsonRejection` handling anywhere in `src/`).

**Repro** (server running on `$PORT`, `$TOKEN` = any account's Bearer token):
```
curl -si -XPOST localhost:$PORT/api/v1/folders -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{not json'
curl -si -XPOST localhost:$PORT/api/v1/files/download -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{"ids":["<uuid>"],"format":"rar"}'
```
**Expected:** 400 (or 422) with `Content-Type: application/json` and body `{"error":{"code":...,"message":...}}`.
**Actual:** `400`/`422` with `Content-Type: text/plain; charset=utf-8`, e.g.
`Failed to deserialize the JSON body into the target type: format: unknown variant `rar`, expected `zip` or `tar.gz` at line 1 column 15`.
Measured on `POST /folders`, `/shares`, `/files/download`, `PATCH /me`, `POST /me/password` with `{not json` (400),
`[]` and `{"name": 5}` (422): all `text/plain`. A client that parses every error as JSON breaks on these.
