//! 聊天核心的流程测试：全部用 `FakeProvider`，不碰网络。

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use super::{AdminEnv, ChatService};
use super::provider::{FakeProvider, MessageRef, Sender, Update};
use crate::config::{
    ChatConfig, ChatLanguage, ChatProviderKind, RegistrationMode, RegistrationPolicy,
};
use crate::db::{Db, test_pool};
use crate::drive::{DriveManager, path_resolver};
use crate::events::ChangeNotifier;
use crate::models::user::User;
use crate::services::user_service::{self, NewAccount, RegisterSource};

struct H {
    svc: ChatService,
    fake: Arc<FakeProvider>,
    db: Db,
    drive: DriveManager,
    /// 数据盘挂载点（临时目录），清空回收站的测试在其下造真实文件。
    mount: PathBuf,
}

/// 聊天运维指令的依赖：接一个临时目录当数据盘。
async fn admin_env() -> (AdminEnv, PathBuf) {
    let mount = std::env::temp_dir().join(format!("ch-chat-{}", uuid::Uuid::new_v4()));
    let drive_manager = DriveManager::new("d");
    drive_manager.set_active(Some(mount.clone())).await;
    let env = AdminEnv {
        drive_manager,
        events: ChangeNotifier::new(),
        trash_retention_days: 7,
    };
    (env, mount)
}

async fn harness() -> H {
    let db = test_pool().await;
    let fake = Arc::new(FakeProvider::default());
    let cfg = ChatConfig {
        provider: ChatProviderKind::Telegram,
        bot_token: None,
        language: ChatLanguage::En,
        mask_identifiers: true,
    };
    let (env, mount) = admin_env().await;
    let drive = env.drive_manager.clone();
    let svc = ChatService::new(db.clone(), &cfg, Some(fake.clone()), env);
    H {
        svc,
        fake,
        db,
        drive,
        mount,
    }
}

fn sender(id: &str) -> Sender {
    Sender {
        id: id.into(),
        label: format!("Admin{id}"),
        chat: id.into(),
        private: true,
    }
}

fn cmd(id: &str, name: &str, args: &str) -> Update {
    Update::Command {
        from: sender(id),
        name: name.into(),
        args: args.into(),
    }
}

fn press(id: &str, token: &str) -> Update {
    Update::ButtonPress {
        from: sender(id),
        message: MessageRef {
            chat: id.into(),
            id: "1".into(),
        },
        token: token.into(),
        ack: format!("ack-{id}"),
    }
}

async fn pair(h: &H, id: &str) {
    let code = h.svc.create_pairing_code().await.unwrap().code;
    h.svc.handle_update(cmd(id, "pair", &code)).await;
}

async fn pending_user(db: &Db, name: &str, email: &str) -> User {
    let policy = RegistrationPolicy {
        mode: RegistrationMode::Approval,
        rate_per_ip_per_hour: 1000,
        pending_max: 1000,
        pending_expire_days: 14,
    };
    user_service::register(
        db,
        &policy,
        NewAccount {
            username: name,
            password: "password123",
            email: Some(email),
            phone: None,
        },
        RegisterSource {
            ip: Some("8.8.8.8".into()),
            is_local: false,
        },
    )
    .await
    .unwrap()
    .0
}

/// 注册一个申请并把通知投递出去。
async fn register_and_deliver(h: &H, name: &str, email: &str) -> User {
    let u = pending_user(&h.db, name, email).await;
    h.svc.notify_registration(&u).await;
    h.svc.deliver_due().await;
    u
}

/// 发给某个会话、带按钮的最后一条消息的 (批准令牌, 拒绝令牌)。
fn tokens_for(h: &H, chat: &str) -> (String, String) {
    let sent = h.fake.sent();
    let (_, m, _) = sent
        .iter()
        .rev()
        .find(|(c, m, _)| c == chat && !m.buttons.is_empty())
        .expect("该管理员应收到带按钮的消息");
    (m.buttons[0].token.clone(), m.buttons[1].token.clone())
}

async fn status_of(h: &H, id: &str) -> (String, i64, Option<String>) {
    let u = user_service::get(&h.db, id).await.unwrap();
    (u.approval_status, u.is_active, u.approval_decided_by)
}

// ---- 运维指令（R-064） ----

/// 开放注册下建一个直接可用的账户。
async fn active_user(db: &Db, name: &str, email: &str) -> User {
    let policy = RegistrationPolicy {
        mode: RegistrationMode::Open,
        rate_per_ip_per_hour: 1000,
        pending_max: 1000,
        pending_expire_days: 14,
    };
    user_service::register(
        db,
        &policy,
        NewAccount {
            username: name,
            password: "password123",
            email: Some(email),
            phone: None,
        },
        RegisterSource {
            ip: Some("127.0.0.1".into()),
            is_local: true,
        },
    )
    .await
    .unwrap()
    .0
}

/// 给账户造一个已进回收站的真实文件（数据库行 + 盘上文件），返回文件路径。
async fn trashed_file(h: &H, owner: &str, name: &str) -> PathBuf {
    sqlx::query(
        "INSERT OR IGNORE INTO drives (id, mount_path, label, is_active, detected_at, last_seen_at) \
         VALUES ('drv', '/x', 'x', 1, 'now', 'now')",
    )
    .execute(&h.db)
    .await
    .unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO files (id, owner_id, parent_id, drive_id, name, file_type, size_bytes, \
         created_at, updated_at, is_deleted) VALUES (?, ?, NULL, 'drv', ?, 'file', 5, 'now', 'now', 0)",
    )
    .bind(&id)
    .bind(owner)
    .bind(name)
    .execute(&h.db)
    .await
    .unwrap();
    let path = path_resolver::file_path(&path_resolver::data_root(&h.mount, "d"), owner, &id);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"hello").unwrap();
    crate::services::trash_service::soft_delete(&h.db, 7, owner, &id)
        .await
        .unwrap();
    path
}

async fn trash_count(h: &H, owner: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM trash WHERE owner_id = ?")
        .bind(owner)
        .fetch_one(&h.db)
        .await
        .unwrap()
}

fn last_text(h: &H) -> String {
    h.fake.sent().last().unwrap().1.text.clone()
}

/// 从确认提示里取出 `/confirm <码>` 的码。
fn code_in(text: &str) -> String {
    text.split("/confirm ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .expect("提示里应有确认码")
        .to_string()
}

async fn is_active(h: &H, id: &str) -> bool {
    user_service::get(&h.db, id).await.unwrap().is_active != 0
}

#[tokio::test]
async fn users_lists_masked_identifiers_and_filters_by_status() {
    let h = harness().await;
    pair(&h, "1").await;
    active_user(&h.db, "Alice", "alice@gmail.com").await;
    pending_user(&h.db, "Bob", "bob@x.org").await;

    h.svc.handle_update(cmd("1", "users", "")).await;
    let all = last_text(&h);
    assert!(all.contains("Alice | a***@gmail.com | approved"), "{all}");
    assert!(all.contains("Bob | b***@x.org | pending"), "{all}");
    assert!(!all.contains("alice@gmail"), "未打码的邮箱不得外发");

    h.svc.handle_update(cmd("1", "users", "pending")).await;
    let only = last_text(&h);
    assert!(only.contains("Bob") && !only.contains("Alice"), "{only}");

    h.svc.handle_update(cmd("1", "users", "bogus")).await;
    assert!(last_text(&h).starts_with("Usage: /users"));
}

#[tokio::test]
async fn status_summarizes_drive_and_accounts() {
    let h = harness().await;
    pair(&h, "1").await;
    active_user(&h.db, "Alice", "alice@gmail.com").await;
    pending_user(&h.db, "Bob", "bob@x.org").await;

    h.svc.handle_update(cmd("1", "status", "")).await;
    let t = last_text(&h);
    assert!(t.contains("Drive: connected"), "{t}");
    assert!(t.contains("1 active, 1 pending, 0 rejected, 0 deleted"), "{t}");

    h.drive.set_active(None).await;
    h.svc.handle_update(cmd("1", "status", "")).await;
    assert!(last_text(&h).contains("Drive: NOT available"));
}

#[tokio::test]
async fn delete_needs_confirmation_then_restore_brings_it_back() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = active_user(&h.db, "Alice", "alice@gmail.com").await;

    h.svc
        .handle_update(cmd("1", "delete", "alice@gmail.com"))
        .await;
    let prompt = last_text(&h);
    assert!(prompt.contains("a***@gmail.com"), "{prompt}");
    assert!(!prompt.contains("IRREVERSIBLE"));
    assert!(is_active(&h, &u.id).await, "确认前什么都不能发生");

    let logs = capture_logs();
    h.svc
        .handle_update(cmd("1", "confirm", &code_in(&prompt).to_lowercase()))
        .await;
    assert!(last_text(&h).starts_with("Deleted \"Alice\""));
    assert!(!is_active(&h, &u.id).await);
    let logged = logs.text();
    assert!(
        logged.contains("admin.user.delete") && logged.contains("chat:telegram:1"),
        "{logged}"
    );

    // 同一个确认不能再用一次。
    h.svc.handle_update(cmd("1", "confirm", &code_in(&prompt))).await;
    assert!(last_text(&h).starts_with("Nothing is waiting"));

    h.svc
        .handle_update(cmd("1", "restore", "alice@gmail.com"))
        .await;
    assert!(last_text(&h).starts_with("Restored \"Alice\""));
    assert!(is_active(&h, &u.id).await);
}

#[tokio::test]
async fn wrong_code_does_nothing_and_cancels_the_pending_operation() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = active_user(&h.db, "Alice", "alice@gmail.com").await;

    h.svc
        .handle_update(cmd("1", "delete", "alice@gmail.com"))
        .await;
    let code = code_in(&last_text(&h));

    let logs = capture_logs();
    h.svc.handle_update(cmd("1", "confirm", "ZZZZZZ")).await;
    assert!(last_text(&h).starts_with("Wrong code"));
    assert!(logs.text().contains("invalid_confirmation"));
    assert!(is_active(&h, &u.id).await);

    // 猜错一次就作废：此后连正确的码也不行。
    h.svc.handle_update(cmd("1", "confirm", &code)).await;
    assert!(last_text(&h).starts_with("Nothing is waiting"));
    assert!(is_active(&h, &u.id).await);
}

#[tokio::test]
async fn expired_confirmation_does_nothing() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = active_user(&h.db, "Alice", "alice@gmail.com").await;

    h.svc
        .handle_update(cmd("1", "delete", "alice@gmail.com"))
        .await;
    let code = code_in(&last_text(&h));
    for c in h.svc.inner.confirmations.lock().unwrap().values_mut() {
        c.expires = std::time::Instant::now();
    }

    h.svc.handle_update(cmd("1", "confirm", &code)).await;
    assert!(last_text(&h).contains("expired"));
    assert!(is_active(&h, &u.id).await);
}

#[tokio::test]
async fn cancel_and_newer_request_replace_the_pending_confirmation() {
    let h = harness().await;
    pair(&h, "1").await;
    let a = active_user(&h.db, "Alice", "alice@gmail.com").await;
    let b = active_user(&h.db, "Bob", "bob@x.org").await;

    h.svc
        .handle_update(cmd("1", "delete", "alice@gmail.com"))
        .await;
    let code_a = code_in(&last_text(&h));
    h.svc.handle_update(cmd("1", "cancel", "")).await;
    h.svc.handle_update(cmd("1", "confirm", &code_a)).await;
    assert!(is_active(&h, &a.id).await);

    // 先申请 Alice 再申请 Bob：旧码作废，只有 Bob 的码执行 Bob。
    h.svc
        .handle_update(cmd("1", "delete", "alice@gmail.com"))
        .await;
    h.svc.handle_update(cmd("1", "delete", "bob@x.org")).await;
    let code_b = code_in(&last_text(&h));
    h.svc.handle_update(cmd("1", "confirm", &code_b)).await;
    assert!(is_active(&h, &a.id).await);
    assert!(!is_active(&h, &b.id).await);
}

#[tokio::test]
async fn confirmation_is_bound_to_the_admin_who_asked() {
    let h = harness().await;
    pair(&h, "1").await;
    pair(&h, "2").await;
    let u = active_user(&h.db, "Alice", "alice@gmail.com").await;

    h.svc
        .handle_update(cmd("1", "delete", "alice@gmail.com"))
        .await;
    let code = code_in(&last_text(&h));

    h.svc.handle_update(cmd("2", "confirm", &code)).await;
    assert!(last_text(&h).starts_with("Nothing is waiting"));
    assert!(is_active(&h, &u.id).await);
}

#[tokio::test]
async fn purge_warns_irreversible_and_only_empties_the_target_trash() {
    let h = harness().await;
    pair(&h, "1").await;
    let a = active_user(&h.db, "Alice", "alice@gmail.com").await;
    let b = active_user(&h.db, "Bob", "bob@x.org").await;
    let file_a = trashed_file(&h, &a.id, "a.txt").await;
    let file_b = trashed_file(&h, &b.id, "b.txt").await;

    h.svc
        .handle_update(cmd("1", "purge", "alice@gmail.com"))
        .await;
    let prompt = last_text(&h);
    assert!(prompt.contains("IRREVERSIBLE"), "{prompt}");
    assert!(file_a.exists() && trash_count(&h, &a.id).await == 1, "确认前不动");

    // 先拔盘再确认：什么都不动。
    let code = code_in(&prompt);
    h.drive.set_active(None).await;
    h.svc.handle_update(cmd("1", "confirm", &code)).await;
    assert!(last_text(&h).contains("drive is not available"));
    assert!(file_a.exists() && trash_count(&h, &a.id).await == 1);

    h.drive.set_active(Some(h.mount.clone())).await;
    h.svc
        .handle_update(cmd("1", "purge", "alice@gmail.com"))
        .await;
    h.svc
        .handle_update(cmd("1", "confirm", &code_in(&last_text(&h))))
        .await;
    assert!(last_text(&h).starts_with("Purged the trash of \"Alice\": 1 item(s)"));
    assert!(!file_a.exists());
    assert_eq!(trash_count(&h, &a.id).await, 0);
    // 另一个账户不受影响。
    assert!(file_b.exists());
    assert_eq!(trash_count(&h, &b.id).await, 1);
}

#[tokio::test]
async fn bad_targets_get_usage_or_not_found_and_stage_nothing() {
    let h = harness().await;
    pair(&h, "1").await;
    h.svc.handle_update(cmd("1", "delete", "")).await;
    assert!(last_text(&h).starts_with("Usage: /delete"));
    h.svc.handle_update(cmd("1", "purge", "ghost@x.org")).await;
    assert!(last_text(&h).starts_with("No such account"));
    h.svc.handle_update(cmd("1", "restore", "ghost@x.org")).await;
    assert!(last_text(&h).starts_with("No such account"));
    assert!(h.svc.inner.confirmations.lock().unwrap().is_empty());

    // 账户未被停用时 /restore 不适用。
    active_user(&h.db, "Alice", "alice@gmail.com").await;
    h.svc
        .handle_update(cmd("1", "restore", "alice@gmail.com"))
        .await;
    assert!(last_text(&h).starts_with("Not done"));
}

#[tokio::test]
async fn unpaired_and_group_senders_cannot_run_admin_commands() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = active_user(&h.db, "Alice", "alice@gmail.com").await;
    let sent_before = h.fake.sent().len();

    for name in ["users", "status", "delete", "restore", "purge", "confirm", "cancel"] {
        h.svc
            .handle_update(cmd("666", name, "alice@gmail.com"))
            .await;
    }
    // 已配对的管理员在群聊里发也不行。
    let mut in_group = sender("1");
    in_group.private = false;
    h.svc
        .handle_update(Update::Command {
            from: in_group,
            name: "delete".into(),
            args: "alice@gmail.com".into(),
        })
        .await;

    assert_eq!(h.fake.sent().len(), sent_before, "没有任何回复");
    assert!(h.svc.inner.confirmations.lock().unwrap().is_empty());
    assert!(is_active(&h, &u.id).await);
}

#[tokio::test]
async fn unpairing_revokes_the_ability_to_confirm() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = active_user(&h.db, "Alice", "alice@gmail.com").await;
    h.svc
        .handle_update(cmd("1", "delete", "alice@gmail.com"))
        .await;
    let code = code_in(&last_text(&h));

    let id = h.svc.status().await.unwrap().admins[0].id.clone();
    h.svc.unpair(&id).await.unwrap();
    let sent = h.fake.sent().len();
    h.svc.handle_update(cmd("1", "confirm", &code)).await;
    assert_eq!(h.fake.sent().len(), sent);
    assert!(is_active(&h, &u.id).await);
}

#[tokio::test]
async fn pairing_code_works_once_and_strangers_get_silence() {
    let h = harness().await;
    let code = h.svc.create_pairing_code().await.unwrap().code;
    assert_eq!(code.len(), 8);

    h.svc
        .handle_update(cmd("1", "pair", &code.to_lowercase()))
        .await;
    assert_eq!(h.svc.status().await.unwrap().admins.len(), 1);
    assert_eq!(h.fake.sent().len(), 1, "成功配对回一条确认");

    // 同一个码第二次（另一发送者）：沉默且不配对。
    h.svc.handle_update(cmd("2", "pair", &code)).await;
    assert_eq!(h.svc.status().await.unwrap().admins.len(), 1);
    // 陌生人的其他指令：沉默。
    h.svc.handle_update(cmd("2", "pending", "")).await;
    h.svc.handle_update(cmd("2", "help", "")).await;
    assert_eq!(h.fake.sent().len(), 1, "陌生人没有收到任何回复");
}

#[tokio::test]
async fn expired_code_is_refused() {
    let h = harness().await;
    let code = h.svc.create_pairing_code().await.unwrap().code;
    sqlx::query("UPDATE chat_pairing_codes SET expires_at = '2000-01-01T00:00:00+00:00'")
        .execute(&h.db)
        .await
        .unwrap();
    h.svc.handle_update(cmd("1", "pair", &code)).await;
    assert!(h.svc.status().await.unwrap().admins.is_empty());
    assert!(h.fake.sent().is_empty());
}

#[tokio::test]
async fn wrong_codes_are_rate_limited_even_for_the_right_code() {
    let h = harness().await;
    let code = h.svc.create_pairing_code().await.unwrap().code;
    for _ in 0..5 {
        h.svc.handle_update(cmd("9", "pair", "WRONGCODE")).await;
    }
    // 试错次数用尽：此后连正确的码也被拒（沉默）。
    h.svc.handle_update(cmd("9", "pair", &code)).await;
    assert!(h.svc.status().await.unwrap().admins.is_empty());
    // 别的发送者不受单人限制影响。
    h.svc.handle_update(cmd("1", "pair", &code)).await;
    assert_eq!(h.svc.status().await.unwrap().admins.len(), 1);
}

#[tokio::test]
async fn pairing_is_unavailable_when_provider_is_none() {
    let db = test_pool().await;
    let cfg = ChatConfig {
        provider: ChatProviderKind::None,
        bot_token: None,
        language: ChatLanguage::En,
        mask_identifiers: true,
    };
    let svc = ChatService::new(db, &cfg, None, admin_env().await.0);
    assert!(svc.create_pairing_code().await.is_err());
    let s = svc.status().await.unwrap();
    assert_eq!((s.provider.as_str(), s.connected), ("none", false));
}

#[tokio::test]
async fn registration_notifies_every_admin_with_masked_identifiers() {
    let h = harness().await;
    pair(&h, "1").await;
    pair(&h, "2").await;
    let before = h.fake.sent().len();

    let u = register_and_deliver(&h, "Alice", "alice@gmail.com").await;

    let sent = h.fake.sent();
    let notes: Vec<_> = sent[before..].iter().collect();
    assert_eq!(notes.len(), 2);
    for (_, m, _) in &notes {
        assert!(m.text.contains("a***@gmail.com"), "{}", m.text);
        assert!(!m.text.contains("alice@"), "未打码的邮箱不得外发");
        assert!(m.text.contains("8.8.8.8"));
        assert_eq!(m.buttons.len(), 2);
        for b in &m.buttons {
            // 不透明：不含用户 id、动作或标识。
            assert_eq!(b.token.len(), 32);
            assert!(!b.token.contains(&u.id) && !b.token.contains("approve"));
        }
    }
    // 两位管理员的令牌互不相同。
    assert_ne!(notes[0].1.buttons[0].token, notes[1].1.buttons[0].token);
}

#[tokio::test]
async fn unmasked_when_configured() {
    let db = test_pool().await;
    let fake = Arc::new(FakeProvider::default());
    let cfg = ChatConfig {
        provider: ChatProviderKind::Telegram,
        bot_token: None,
        language: ChatLanguage::ZhHans,
        mask_identifiers: false,
    };
    let (env, mount) = admin_env().await;
    let drive = env.drive_manager.clone();
    let h = H {
        svc: ChatService::new(db.clone(), &cfg, Some(fake.clone()), env),
        fake,
        db,
        drive,
        mount,
    };
    pair(&h, "1").await;
    register_and_deliver(&h, "Bob", "bob@x.org").await;
    let sent = h.fake.sent();
    let text = &sent.last().unwrap().1.text;
    assert!(
        text.contains("bob@x.org") && text.contains("新的注册申请"),
        "{text}"
    );
}

#[tokio::test]
async fn paired_admin_approves_and_message_is_edited() {
    let h = harness().await;
    pair(&h, "1").await;
    pair(&h, "2").await;
    let u = register_and_deliver(&h, "Alice", "alice@gmail.com").await;
    let (approve, _) = tokens_for(&h, "1");

    h.svc.handle_update(press("1", &approve)).await;

    let (status, active, by) = status_of(&h, &u.id).await;
    assert_eq!((status.as_str(), active), ("approved", 1));
    assert_eq!(by.as_deref(), Some("chat:telegram:1"));
    assert_eq!(h.fake.acks(), vec!["ack-1".to_string()]);

    // 两位管理员的原消息都被编辑：按钮移除，写明决定者。
    let edits = h.fake.edits();
    assert_eq!(edits.len(), 2);
    for (_, m) in &edits {
        assert!(m.buttons.is_empty());
        assert!(m.text.contains("Approved by Admin1"), "{}", m.text);
        assert!(m.text.contains("a***@gmail.com"));
    }

    // 重放同一令牌：不再生效，并告知令牌无效。
    let replies = h.fake.sent().len();
    h.svc.handle_update(press("1", &approve)).await;
    assert_eq!(h.fake.edits().len(), 2);
    assert!(h.fake.sent().len() > replies);
    assert!(
        h.fake
            .sent()
            .last()
            .unwrap()
            .1
            .text
            .contains("no longer valid")
    );
}

#[tokio::test]
async fn reject_button_rejects() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = register_and_deliver(&h, "Eve", "eve@x.org").await;
    let (_, reject) = tokens_for(&h, "1");
    h.svc.handle_update(press("1", &reject)).await;
    let (status, active, _) = status_of(&h, &u.id).await;
    assert_eq!((status.as_str(), active), ("rejected", 0));
    assert!(h.fake.edits()[0].1.text.contains("Rejected by"));
}

#[tokio::test]
async fn unpaired_sender_cannot_press_and_is_audited() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = register_and_deliver(&h, "Alice", "alice@gmail.com").await;
    let (approve, _) = tokens_for(&h, "1");
    let sent_before = h.fake.sent().len();

    let logs = capture_logs();
    h.svc.handle_update(press("666", &approve)).await;
    let logged = logs.text();

    assert_eq!(status_of(&h, &u.id).await.0, "pending");
    assert_eq!(h.fake.sent().len(), sent_before, "陌生人没有任何回复");
    assert!(h.fake.acks().is_empty());
    assert!(
        logged.contains("unauthorized") && logged.contains("666"),
        "{logged}"
    );
    // 令牌未被消耗：真正的管理员仍可使用。
    h.svc.handle_update(press("1", &approve)).await;
    assert_eq!(status_of(&h, &u.id).await.0, "approved");
}

#[tokio::test]
async fn token_bound_to_another_admin_is_refused() {
    let h = harness().await;
    pair(&h, "1").await;
    pair(&h, "2").await;
    let u = register_and_deliver(&h, "Alice", "alice@gmail.com").await;
    let (approve_of_1, _) = tokens_for(&h, "1");

    h.svc.handle_update(press("2", &approve_of_1)).await;
    assert_eq!(status_of(&h, &u.id).await.0, "pending");
    assert!(
        h.fake
            .sent()
            .last()
            .unwrap()
            .1
            .text
            .contains("no longer valid")
    );
}

#[tokio::test]
async fn expired_token_is_refused() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = register_and_deliver(&h, "Alice", "alice@gmail.com").await;
    let (approve, _) = tokens_for(&h, "1");
    sqlx::query("UPDATE chat_action_tokens SET expires_at = '2000-01-01T00:00:00+00:00'")
        .execute(&h.db)
        .await
        .unwrap();
    h.svc.handle_update(press("1", &approve)).await;
    assert_eq!(status_of(&h, &u.id).await.0, "pending");
}

#[tokio::test]
async fn simultaneous_presses_produce_exactly_one_decision() {
    let h = harness().await;
    pair(&h, "1").await;
    pair(&h, "2").await;
    let u = register_and_deliver(&h, "Alice", "alice@gmail.com").await;
    let (approve, _) = tokens_for(&h, "1");
    let (_, reject) = tokens_for(&h, "2");

    tokio::join!(
        h.svc.handle_update(press("1", &approve)),
        h.svc.handle_update(press("2", &reject)),
    );

    let (status, _, by) = status_of(&h, &u.id).await;
    assert_ne!(status, "pending");
    let winner = by.unwrap();
    assert!(winner == "chat:telegram:1" || winner == "chat:telegram:2");
    // 输家收到「已被处理」，且点名决定者。
    let losers: Vec<_> = h
        .fake
        .sent()
        .into_iter()
        .filter(|(_, m, _)| m.text.starts_with("Already decided"))
        .collect();
    assert_eq!(losers.len(), 1);
    let loser_chat = if winner.ends_with(":1") { "2" } else { "1" };
    assert_eq!(losers[0].0, loser_chat);
}

#[tokio::test]
async fn button_after_decision_elsewhere_reports_who_decided() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = register_and_deliver(&h, "Alice", "alice@gmail.com").await;
    let (approve, _) = tokens_for(&h, "1");
    crate::services::approval_service::decide(
        &h.db,
        &u.id,
        crate::services::approval_service::Decision::Reject,
        "admin-endpoint",
        None,
        true,
    )
    .await
    .unwrap();

    h.svc.handle_update(press("1", &approve)).await;
    assert_eq!(status_of(&h, &u.id).await.0, "rejected");
    let last = h.fake.sent().last().unwrap().1.text.clone();
    assert!(
        last.starts_with("Already decided") && last.contains("admin-endpoint"),
        "{last}"
    );
}

#[tokio::test]
async fn unpairing_revokes_immediately() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = register_and_deliver(&h, "Alice", "alice@gmail.com").await;
    let (approve, _) = tokens_for(&h, "1");
    let admin_id = h.svc.status().await.unwrap().admins[0].id.clone();

    h.svc.unpair(&admin_id).await.unwrap();
    assert!(matches!(
        h.svc.unpair(&admin_id).await,
        Err(crate::error::AppError::NotFound)
    ));

    let sent_before = h.fake.sent().len();
    h.svc.handle_update(press("1", &approve)).await;
    h.svc.handle_update(cmd("1", "pending", "")).await;
    assert_eq!(status_of(&h, &u.id).await.0, "pending");
    assert_eq!(h.fake.sent().len(), sent_before);
}

#[tokio::test]
async fn group_chats_are_ignored_even_for_admins() {
    let h = harness().await;
    pair(&h, "1").await;
    let before = h.fake.sent().len();
    let mut from = sender("1");
    from.private = false;
    h.svc
        .handle_update(Update::Command {
            from,
            name: "whoami".into(),
            args: String::new(),
        })
        .await;
    assert_eq!(h.fake.sent().len(), before);
}

#[tokio::test]
async fn chat_outage_does_not_block_registration_and_outbox_retries() {
    let h = harness().await;
    pair(&h, "1").await;
    h.fake.set_down(true);

    let u = pending_user(&h.db, "Alice", "alice@gmail.com").await;
    h.svc.notify_registration(&u).await;
    h.svc.deliver_due().await;
    assert_eq!(
        status_of(&h, &u.id).await.0,
        "pending",
        "宕机期间不会隐式批准"
    );
    let (attempts, state): (i64, String) =
        sqlx::query_as("SELECT attempts, state FROM chat_outbox WHERE kind = 'notify'")
            .fetch_one(&h.db)
            .await
            .unwrap();
    assert_eq!((attempts, state.as_str()), (1, "queued"));

    // 退避期内不会重试；到期且聊天恢复后送达。
    h.fake.set_down(false);
    h.svc.deliver_due().await;
    assert_eq!(h.fake.sent().len(), 1, "只有配对确认");
    make_due(&h.db).await;
    h.svc.deliver_due().await;
    assert_eq!(h.fake.sent().len(), 2);
    assert_eq!(status_of(&h, &u.id).await.0, "pending");
}

#[tokio::test]
async fn delivery_gives_up_after_max_attempts_and_audits() {
    let h = harness().await;
    pair(&h, "1").await;
    h.fake.set_down(true);
    let u = pending_user(&h.db, "Alice", "alice@gmail.com").await;
    h.svc.notify_registration(&u).await;

    let logs = capture_logs();
    for _ in 0..8 {
        make_due(&h.db).await;
        h.svc.deliver_due().await;
    }
    let (state,): (String,) = sqlx::query_as("SELECT state FROM chat_outbox WHERE kind = 'notify'")
        .fetch_one(&h.db)
        .await
        .unwrap();
    assert_eq!(state, "failed");
    assert!(logs.text().contains("chat.delivery"));
    // 放弃后按钮令牌不再留在库里的待发消息中。
    let (buttons,): (Option<String>,) =
        sqlx::query_as("SELECT buttons FROM chat_outbox WHERE kind = 'notify'")
            .fetch_one(&h.db)
            .await
            .unwrap();
    assert!(buttons.is_none());
}

async fn make_due(db: &Db) {
    sqlx::query("UPDATE chat_outbox SET next_attempt_at = '2000-01-01T00:00:00+00:00' WHERE state = 'queued'")
        .execute(db)
        .await
        .unwrap();
}

#[tokio::test]
async fn bursts_are_coalesced_into_one_summary() {
    let h = harness().await;
    pair(&h, "1").await;
    let before = h.fake.sent().len();
    for i in 0..8 {
        let u = pending_user(&h.db, "User", &format!("u{i}@x.org")).await;
        h.svc.notify_registration(&u).await;
    }
    h.svc.deliver_due().await;
    h.svc.deliver_due().await;

    let sent = h.fake.sent();
    let new: Vec<_> = sent[before..].iter().map(|(_, m, _)| m).collect();
    assert_eq!(new.iter().filter(|m| !m.buttons.is_empty()).count(), 5);
    let summaries: Vec<_> = new.iter().filter(|m| m.buttons.is_empty()).collect();
    assert_eq!(summaries.len(), 1, "窗口内只有一条摘要");
    assert!(summaries[0].text.contains("/pending"));
}

#[tokio::test]
async fn daily_reminder_is_sent_once_per_day_for_stale_pending() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = pending_user(&h.db, "Old", "old@x.org").await;

    // 刚申请的不提醒。
    h.svc.remind_if_due().await;
    h.svc.deliver_due().await;
    assert_eq!(h.fake.sent().len(), 1);

    let old = (chrono::Utc::now() - chrono::Duration::hours(30)).to_rfc3339();
    sqlx::query("UPDATE users SET created_at = ? WHERE id = ?")
        .bind(&old)
        .bind(&u.id)
        .execute(&h.db)
        .await
        .unwrap();
    h.svc.remind_if_due().await;
    h.svc.remind_if_due().await;
    h.svc.deliver_due().await;
    let sent = h.fake.sent();
    assert_eq!(sent.len(), 2, "同一天只提醒一次");
    assert!(sent[1].1.text.contains("Reminder") && sent[1].1.text.contains("/pending"));
}

#[tokio::test]
async fn queued_notification_is_dropped_if_account_was_decided_meanwhile() {
    let h = harness().await;
    pair(&h, "1").await;
    let u = pending_user(&h.db, "Alice", "alice@gmail.com").await;
    h.svc.notify_registration(&u).await;
    crate::services::approval_service::decide(
        &h.db,
        &u.id,
        crate::services::approval_service::Decision::Approve,
        "admin-endpoint",
        None,
        true,
    )
    .await
    .unwrap();
    h.svc.deliver_due().await;
    assert_eq!(h.fake.sent().len(), 1, "只有配对确认");
}

#[tokio::test]
async fn commands_reply_to_paired_admins() {
    let h = harness().await;
    pair(&h, "1").await;

    h.svc.handle_update(cmd("1", "whoami", "")).await;
    assert!(h.fake.sent().last().unwrap().1.text.contains("Admin1"));
    h.svc.handle_update(cmd("1", "start", "")).await;
    assert!(h.fake.sent().last().unwrap().1.text.contains("/pending"));
    h.svc.handle_update(cmd("1", "pending", "")).await;
    assert!(
        h.fake
            .sent()
            .last()
            .unwrap()
            .1
            .text
            .contains("No registration requests")
    );

    // /pending 为每个待批准申请发一条带新令牌的消息，且能直接用来批准。
    let u = pending_user(&h.db, "Alice", "alice@gmail.com").await;
    let before = h.fake.sent().len();
    h.svc.handle_update(cmd("1", "pending", "")).await;
    let sent = h.fake.sent();
    assert_eq!(sent.len(), before + 2, "标题 + 一条申请");
    let (approve, _) = tokens_for(&h, "1");
    h.svc.handle_update(press("1", &approve)).await;
    assert_eq!(status_of(&h, &u.id).await.0, "approved");
}

#[tokio::test]
async fn poll_once_feeds_updates_to_the_dispatcher() {
    let h = harness().await;
    let code = h.svc.create_pairing_code().await.unwrap().code;
    h.fake.push_update(cmd("7", "pair", &code));
    assert!(h.svc.poll_once().await);
    assert_eq!(h.svc.status().await.unwrap().admins.len(), 1);
    assert!(h.svc.status().await.unwrap().connected);
}

#[tokio::test]
async fn no_secret_or_unmasked_identifier_reaches_the_logs() {
    let h = harness().await;
    let logs = capture_logs();
    let code = h.svc.create_pairing_code().await.unwrap().code;
    h.svc.handle_update(cmd("1", "pair", &code)).await;
    let u = register_and_deliver(&h, "Alice", "alice@gmail.com").await;
    let (approve, reject) = tokens_for(&h, "1");
    h.svc.handle_update(cmd("5", "pair", "BADBADBA")).await;
    h.svc.handle_update(press("1", &approve)).await;

    let logged = logs.text();
    assert!(!logged.contains("alice@gmail.com"), "邮箱不得进日志");
    assert!(!logged.contains(&code), "配对码不得进日志");
    assert!(
        !logged.contains(&approve) && !logged.contains(&reject),
        "令牌不得进日志"
    );
    assert!(!logged.contains("BADBADBA"), "指令参数不得进日志");
    assert!(logged.contains("user.approve"), "{logged}");
    assert!(logged.contains(&u.username));
}

// ---- 日志捕获 ----
//
// 全进程只装一次全局订阅者，把输出写进「当前线程」的缓冲区：每个测试跑在自己的线程上
// （tokio::test 是单线程运行时），互不串扰；也避开多个线程各自 set_default 时
// tracing 的 callsite 兴趣缓存导致事件被丢的问题。

thread_local! {
    static LOG_BUF: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

struct ThreadBufWriter;

impl io::Write for ThreadBufWriter {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        LOG_BUF.with(|buf| buf.borrow_mut().extend_from_slice(b));
        Ok(b.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct LogCapture;

impl LogCapture {
    fn text(&self) -> String {
        LOG_BUF.with(|buf| String::from_utf8_lossy(&buf.borrow()).into_owned())
    }
}

fn capture_logs() -> LogCapture {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let subscriber = tracing_subscriber::fmt()
            .with_writer(|| ThreadBufWriter)
            .with_ansi(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
    });
    LOG_BUF.with(|buf| buf.borrow_mut().clear());
    LogCapture
}
