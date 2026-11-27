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

## Notes (not bugs)

- `cloud_home` is a binary-only crate and `FakeProvider` exists only under `cfg(test)`, so the integration tests in
  `tests/` cannot link against it. `tests/chat_fake.rs` therefore re-compiles the same sources with `#[path]` (see the
  header comment there). Splitting `src/` into `lib.rs` + a thin `main.rs` would remove that boilerplate
  and also stop the mirrored unit tests from running twice.
- `cargo test` reports `lifecycle.rs: method wait_idle is never used` (a pre-existing idle-code warning, R-070).
