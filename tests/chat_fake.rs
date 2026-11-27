//! 审批 + 聊天的集成测试（R-066，spec/admin_chat.md §8 第 7–10 条）：
//! 真实的路由、数据库、迁移、后台收发循环，聊天提供方用 `FakeProvider`。
//!
//! 为什么要把 `src/` 的模块「镜像」进来：`cloud_home` 是纯二进制 crate（没有 `lib.rs`），
//! 集成测试无法 `use` 它的类型，而 `FakeProvider` 又只在 `cfg(test)` 下存在，真实二进制里没有。
//! 所以这里用 `#[path]` 把同一份源码按相同的模块树编进测试 crate（`cfg(test)` 为真，
//! `FakeProvider` 可用），只通过公开 API 驱动：`ChatService::new/spawn`、HTTP 路由、`FakeProvider`。
//! 副作用：被镜像进来的单元测试也会在本二进制里再跑一遍，所以用
//! `cargo test --test chat_fake fake_chat::` 只跑本文件的测试。
//! 若以后把 `src/` 拆成 lib + bin，这一段 `#[path]` 样板即可删除。
#![allow(dead_code, unused_imports)]

#[path = "../src/auth/mod.rs"]
pub mod auth;
#[path = "../src/chat/mod.rs"]
pub mod chat;
#[path = "../src/config.rs"]
pub mod config;
#[path = "../src/db/mod.rs"]
pub mod db;
#[path = "../src/drive/mod.rs"]
pub mod drive;
#[path = "../src/error.rs"]
pub mod error;
#[path = "../src/events.rs"]
pub mod events;
#[path = "../src/handlers/mod.rs"]
pub mod handlers;
#[path = "../src/lifecycle.rs"]
pub mod lifecycle;
#[path = "../src/logging.rs"]
pub mod logging;
#[path = "../src/models/mod.rs"]
pub mod models;
#[path = "../src/openapi.rs"]
pub mod openapi;
#[path = "../src/services/mod.rs"]
pub mod services;
#[path = "../src/state.rs"]
pub mod state;
#[path = "../src/tasks/mod.rs"]
pub mod tasks;
#[path = "../src/update/mod.rs"]
pub mod update;
#[path = "../src/util.rs"]
pub mod util;
#[path = "../src/version.rs"]
pub mod version;

mod fake_chat {
    use std::future::Future;
    use std::io;
    use std::net::SocketAddr;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use axum::Router;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{Request, header};
    use serde_json::{Value, json};
    use tokio_util::sync::CancellationToken;
    use tower::ServiceExt;

    use crate::chat::provider::{
        BoxFut, ChatProvider, FakeProvider, Message, MessageRef, Sender, Update,
    };
    use crate::chat::{AdminEnv, ChatService};
    use crate::config::{
        ChatConfig, ChatLanguage, ChatProviderKind, Config, RegistrationMode, RegistrationPolicy,
    };
    use crate::db::Db;
    use crate::state::AppState;
    use crate::update::settings::{Channel, Mode};

    const ADMIN_PW: &str = "adm1n-secret-pw";
    const PASSWORD: &str = "correct-horse-battery";
    const REMOTE: &str = "8.8.8.8";
    const LOCAL: &str = "127.0.0.1";

    // ---- 日志捕获：测试在单线程运行时里跑，用线程级订阅者，不碰全局订阅者 ----

    #[derive(Clone, Default)]
    struct Logs(Arc<Mutex<Vec<u8>>>);

    impl Logs {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
    }

    struct LogWriter(Logs);

    impl io::Write for LogWriter {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            (self.0).0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    // ---- 提供方：FakeProvider + 一个让出调度的拉取，避免轮询循环空转 ----

    /// 把调用转给 `FakeProvider`；`next_updates` 先睡 5ms，像真实长轮询一样让出线程。
    struct Ticking(Arc<FakeProvider>);

    impl ChatProvider for Ticking {
        fn send<'a>(&'a self, chat: &'a str, msg: &'a Message) -> BoxFut<'a, MessageRef> {
            self.0.send(chat, msg)
        }

        fn edit<'a>(&'a self, target: &'a MessageRef, msg: &'a Message) -> BoxFut<'a, ()> {
            self.0.edit(target, msg)
        }

        fn ack_button<'a>(&'a self, ack: &'a str) -> BoxFut<'a, ()> {
            self.0.ack_button(ack)
        }

        fn next_updates(&self) -> BoxFut<'_, Vec<Update>> {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(5)).await;
                self.0.next_updates().await
            })
        }
    }

    // ---- 环境 ----

    struct Env {
        app: Router,
        db: Db,
        fake: Arc<FakeProvider>,
        logs: Logs,
        shutdown: CancellationToken,
    }

    impl Drop for Env {
        fn drop(&mut self) {
            self.shutdown.cancel();
        }
    }

    /// 在一个全新的环境里跑场景：单线程运行时 + 线程级日志订阅者。
    fn scenario<F, Fut>(body: F)
    where
        F: FnOnce(Env) -> Fut,
        Fut: Future<Output = ()>,
    {
        let logs = Logs::default();
        let writer_logs = logs.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || LogWriter(writer_logs.clone()))
            .with_ansi(false)
            .finish();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        tracing::subscriber::with_default(subscriber, || {
            rt.block_on(async {
                let env = Env::new(logs).await;
                body(env).await;
            });
        });
    }

    impl Env {
        async fn new(logs: Logs) -> Self {
            let root = std::env::temp_dir().join(format!("ch-chat-fake-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            let chat_cfg = ChatConfig {
                provider: ChatProviderKind::Telegram,
                bot_token: None,
                language: ChatLanguage::En,
                mask_identifiers: true,
            };
            let config = Arc::new(Config {
                server_host: LOCAL.into(),
                server_port: 0,
                volumes_watch_path: PathBuf::from("/nonexistent"),
                drive_data_dir: "d".into(),
                jwt_secret: "s".into(),
                jwt_expires_in_hours: 1,
                max_upload_size_mb: 1,
                trash_retention_days: 7,
                trash_cleanup_interval_hours: 1,
                cors_allowed_origins: vec![],
                public_base_url: "http://localhost".into(),
                log_dir: root.clone(),
                log_format: "pretty".into(),
                log_file_enabled: false,
                log_max_size_mb: 1,
                trust_proxy_headers: false,
                admin_password: Some(ADMIN_PW.into()),
                update_mode: Mode::Manual,
                update_window: "03:00-05:00".into(),
                update_channel: Channel::Stable,
                update_check_interval_hours: 6,
                update_root: root,
                update_base_url: None,
                registration: RegistrationPolicy {
                    mode: RegistrationMode::Approval,
                    rate_per_ip_per_hour: 1000,
                    pending_max: 1000,
                    pending_expire_days: 14,
                },
                chat: chat_cfg.clone(),
            });
            let db = crate::db::test_pool().await;
            let drive_manager = crate::drive::DriveManager::new("d");
            let activity = crate::lifecycle::ActivityTracker::new();
            let shutdown = CancellationToken::new();
            let events = crate::events::ChangeNotifier::new();
            let updater = crate::update::Updater::new(
                config.clone(),
                db.clone(),
                drive_manager.clone(),
                activity.clone(),
                shutdown.clone(),
                events.clone(),
            )
            .unwrap();
            let fake = Arc::new(FakeProvider::default());
            let chat = ChatService::new(
                db.clone(),
                &chat_cfg,
                Some(Arc::new(Ticking(fake.clone()))),
                AdminEnv {
                    drive_manager: drive_manager.clone(),
                    events: events.clone(),
                    trash_retention_days: 7,
                },
            );
            chat.spawn(shutdown.clone());
            let state = AppState {
                db: db.clone(),
                config,
                drive_manager,
                drive_id: "d".into(),
                events,
                activity,
                shutdown: shutdown.clone(),
                updater,
                chat,
            };
            Self {
                app: crate::handlers::build_router(state),
                db,
                fake,
                logs,
                shutdown,
            }
        }

        async fn call(
            &self,
            method: &str,
            path: &str,
            body: Option<Value>,
            admin_pw: Option<&str>,
            peer: &str,
        ) -> (u16, Value) {
            let mut req = Request::builder()
                .method(method)
                .uri(format!("/api/v1{path}"));
            if let Some(p) = admin_pw {
                req = req.header("x-admin-password", p);
            }
            let body = match body {
                Some(v) => {
                    req = req.header(header::CONTENT_TYPE, "application/json");
                    Body::from(v.to_string())
                }
                None => Body::empty(),
            };
            let mut req = req.body(body).unwrap();
            let peer: SocketAddr = format!("{peer}:50000").parse().unwrap();
            req.extensions_mut().insert(ConnectInfo(peer));
            let resp = self.app.clone().oneshot(req).await.unwrap();
            let status = resp.status().as_u16();
            let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
            (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
        }

        async fn admin(&self, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
            self.call(method, path, body, Some(ADMIN_PW), LOCAL).await
        }

        /// 注册（默认从公网来，所以是 pending）。
        async fn register(&self, name: &str, email: &str) -> u16 {
            self.register_from(name, email, None, REMOTE).await
        }

        async fn register_from(&self, name: &str, email: &str, phone: Option<&str>, peer: &str) -> u16 {
            let mut body = json!({"username": name, "password": PASSWORD, "email": email});
            if let Some(p) = phone {
                body["phone"] = json!(p);
            }
            self.call("POST", "/auth/register", Some(body), None, peer).await.0
        }

        /// 引导账户：本机第一个注册，直接批准。
        async fn owner(&self) {
            assert_eq!(self.register_from("Owner", "owner@home.test", None, LOCAL).await, 201);
        }

        /// `(status, error code)`。
        async fn login(&self, email: &str) -> (u16, String) {
            let (s, body) = self
                .call(
                    "POST",
                    "/auth/login",
                    Some(json!({"identifier": email, "password": PASSWORD})),
                    None,
                    LOCAL,
                )
                .await;
            (s, body["error"]["code"].as_str().unwrap_or("").to_string())
        }

        async fn user(&self, email: &str) -> Value {
            let (_, list) = self.admin("GET", "/admin/users", None).await;
            list["data"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|u| u["identities"].as_array().unwrap().iter().any(|i| i["identifier"] == email))
                .unwrap_or_else(|| panic!("找不到 {email}"))
                .clone()
        }

        async fn status_of(&self, email: &str) -> String {
            self.user(email).await["approval_status"].as_str().unwrap().to_string()
        }

        /// 签发配对码、让 `id` 发 `/pair`，返回新管理员在库里的 id。
        async fn pair(&self, id: &str) -> String {
            let (s, body) = self.admin("POST", "/admin/chat/pairing", None).await;
            assert_eq!(s, 200);
            let code = body["data"]["code"].as_str().unwrap().to_string();
            self.fake.push_update(cmd(id, "pair", &code));
            let label = format!("Admin{id}");
            eventually(|| async {
                self.admins().await.iter().any(|a| a["label"] == label.as_str())
            })
            .await;
            let admins = self.admins().await;
            admins.iter().find(|a| a["label"] == label.as_str()).unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string()
        }

        async fn admins(&self) -> Vec<Value> {
            let (_, body) = self.admin("GET", "/admin/chat", None).await;
            body["data"]["admins"].as_array().unwrap().clone()
        }

        /// 等到 `chat` 收到带按钮的最新消息，返回 (批准令牌, 拒绝令牌)。
        async fn tokens_for(&self, chat: &str, expect_messages: usize) -> (String, String) {
            eventually(|| async { self.buttoned(chat).len() >= expect_messages }).await;
            let m = self.buttoned(chat).pop().unwrap();
            (m.buttons[0].token.clone(), m.buttons[1].token.clone())
        }

        fn buttoned(&self, chat: &str) -> Vec<Message> {
            self.fake
                .sent()
                .into_iter()
                .filter(|(c, m, _)| c == chat && !m.buttons.is_empty())
                .map(|(_, m, _)| m)
                .collect()
        }

        /// 屏障：已配对的 `admin` 发一条 `/whoami`，等到它的回复出现——
        /// 轮询循环按序处理更新，所以此前推入的更新都已处理完。
        async fn barrier(&self, admin: &str) {
            let before = self.whoami_replies(admin);
            self.fake.push_update(cmd(admin, "whoami", ""));
            eventually(|| async { self.whoami_replies(admin) > before }).await;
        }

        fn whoami_replies(&self, admin: &str) -> usize {
            self.fake
                .sent()
                .iter()
                .filter(|(c, m, _)| c == admin && m.text.contains("You are admin"))
                .count()
        }

        /// 某事件、某用户名在日志里出现了几次。
        fn audit_count(&self, event: &str, username: &str) -> usize {
            self.logs
                .text()
                .lines()
                .filter(|l| l.contains(&format!("event=\"{event}\"")) && l.contains(&format!("target_username={username}")))
                .count()
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

    async fn eventually<F, Fut>(mut cond: F)
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = bool>,
    {
        for _ in 0..600 {
            if cond().await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("条件在 6 秒内没有成立");
    }

    // ---- 验收 7：通知、按钮、授权 ----

    #[test]
    fn registration_notifies_every_admin_masked_and_a_press_approves_end_to_end() {
        scenario(|env| async move {
            env.owner().await;
            env.pair("1").await;
            env.pair("2").await;

            let status = env
                .register_from("Alice", "alice@gmail.com", Some("+886912345123"), REMOTE)
                .await;
            assert_eq!(status, 202);

            let (approve1, reject1) = env.tokens_for("1", 1).await;
            let (approve2, _) = env.tokens_for("2", 1).await;
            for chat in ["1", "2"] {
                let text = env.buttoned(chat)[0].text.clone();
                assert!(text.contains("a***@gmail.com"), "{text}");
                assert!(text.contains("+886******123"), "{text}");
                assert!(!text.contains("alice@") && !text.contains("912345"), "{text}");
                assert!(text.contains(REMOTE), "{text}");
            }
            for t in [&approve1, &reject1, &approve2] {
                assert_eq!(t.len(), 32);
                assert!(t.bytes().all(|b| b.is_ascii_hexdigit()));
            }
            assert_ne!(approve1, approve2, "每位管理员各有自己的令牌");

            // 批准之前：登录被拒、没有令牌。
            assert_eq!(env.login("alice@gmail.com").await, (403, "ACCOUNT_PENDING".into()));

            env.fake.push_update(press("1", &approve1));
            eventually(|| async { env.status_of("alice@gmail.com").await == "approved" }).await;
            assert_eq!(env.login("alice@gmail.com").await.0, 200);
            assert_eq!(env.user("alice@gmail.com").await["approval_decided_by"], "chat:telegram:1");
            assert_eq!(env.fake.acks(), vec!["ack-1".to_string()]);

            // 两位管理员的消息都被改写：按钮移除，写明决定者。
            eventually(|| async { env.fake.edits().len() == 2 }).await;
            for (_, m) in env.fake.edits() {
                assert!(m.buttons.is_empty());
                assert!(m.text.contains("Approved by Admin1"), "{}", m.text);
            }
        });
    }

    #[test]
    fn unpaired_sender_is_ignored_audited_and_does_not_burn_the_token() {
        scenario(|env| async move {
            env.owner().await;
            env.pair("1").await;
            env.register("Alice", "alice@gmail.com").await;
            let (approve, _) = env.tokens_for("1", 1).await;
            let sent = env.fake.sent().len();

            env.fake.push_update(press("666", &approve));
            env.fake.push_update(cmd("666", "pending", ""));
            env.fake.push_update(cmd("666", "delete", "alice@gmail.com"));
            env.barrier("1").await;

            assert_eq!(env.status_of("alice@gmail.com").await, "pending");
            assert_eq!(env.fake.sent().len(), sent + 1, "陌生人没有任何回复（只有屏障的回复）");
            assert!(env.fake.acks().is_empty());
            let logs = env.logs.text();
            assert!(logs.contains("unauthorized") && logs.contains("sender_id=666"), "{logs}");

            // 令牌没被陌生人消耗：真正的管理员仍可使用。
            env.fake.push_update(press("1", &approve));
            eventually(|| async { env.status_of("alice@gmail.com").await == "approved" }).await;
        });
    }

    #[test]
    fn forged_and_guessed_tokens_are_refused() {
        scenario(|env| async move {
            env.owner().await;
            env.pair("1").await;
            env.register("Alice", "alice@gmail.com").await;
            let (approve, _) = env.tokens_for("1", 1).await;
            let stored: (String,) = sqlx::query_as("SELECT token_hash FROM chat_action_tokens LIMIT 1")
                .fetch_one(&env.db)
                .await
                .unwrap();

            // 随机猜、空串、改一位、库里存的摘要本身（拿到数据库也不能直接当令牌用）。
            let mut flipped = approve.clone();
            flipped.replace_range(0..1, if approve.starts_with('0') { "1" } else { "0" });
            for forged in ["0123456789abcdef0123456789abcdef", "", flipped.as_str(), stored.0.as_str()] {
                env.fake.push_update(press("1", forged));
                env.barrier("1").await;
                assert_eq!(env.status_of("alice@gmail.com").await, "pending", "{forged:?}");
            }
            let invalid = env
                .fake
                .sent()
                .iter()
                .filter(|(c, m, _)| c == "1" && m.text.contains("no longer valid"))
                .count();
            assert_eq!(invalid, 4);
            assert_eq!(env.login("alice@gmail.com").await.1, "ACCOUNT_PENDING");
        });
    }

    #[test]
    fn a_replayed_token_decides_nothing_the_second_time() {
        scenario(|env| async move {
            env.owner().await;
            env.pair("1").await;
            env.register("Alice", "alice@gmail.com").await;
            let (approve, reject) = env.tokens_for("1", 1).await;

            env.fake.push_update(press("1", &approve));
            eventually(|| async { env.status_of("alice@gmail.com").await == "approved" }).await;
            let edits = env.fake.edits().len();

            // 同一个令牌再按：无效；同一条消息上的另一个令牌：账户已决定，什么都不执行。
            env.fake.push_update(press("1", &approve));
            env.barrier("1").await;
            assert!(env.fake.sent().iter().any(|(_, m, _)| m.text.contains("no longer valid")));
            env.fake.push_update(press("1", &reject));
            env.barrier("1").await;
            assert!(env.fake.sent().iter().any(|(_, m, _)| m.text.starts_with("Already decided")));

            assert_eq!(env.status_of("alice@gmail.com").await, "approved");
            assert_eq!(env.fake.edits().len(), edits, "重放不会再改消息");
            assert_eq!(env.audit_count("user.approve", "Alice"), 1);
            assert_eq!(env.audit_count("user.reject", "Alice"), 0);
        });
    }

    #[test]
    fn an_expired_token_is_refused() {
        scenario(|env| async move {
            env.owner().await;
            env.pair("1").await;
            env.register("Alice", "alice@gmail.com").await;
            let (approve, _) = env.tokens_for("1", 1).await;
            sqlx::query("UPDATE chat_action_tokens SET expires_at = '2000-01-01T00:00:00+00:00'")
                .execute(&env.db)
                .await
                .unwrap();

            env.fake.push_update(press("1", &approve));
            env.barrier("1").await;
            assert_eq!(env.status_of("alice@gmail.com").await, "pending");
            assert!(env.fake.sent().iter().any(|(_, m, _)| m.text.contains("no longer valid")));
            assert_eq!(env.login("alice@gmail.com").await.1, "ACCOUNT_PENDING");
        });
    }

    #[test]
    fn a_token_issued_to_one_admin_cannot_be_used_by_another() {
        scenario(|env| async move {
            env.owner().await;
            env.pair("1").await;
            env.pair("2").await;
            env.register("Alice", "alice@gmail.com").await;
            let (approve_of_1, _) = env.tokens_for("1", 1).await;
            env.tokens_for("2", 1).await;

            env.fake.push_update(press("2", &approve_of_1));
            env.barrier("2").await;
            assert_eq!(env.status_of("alice@gmail.com").await, "pending");
            assert!(env.fake.sent().iter().any(|(c, m, _)| c == "2" && m.text.contains("no longer valid")));

            // 令牌没被「偷用」消耗：属主仍可用。
            env.fake.push_update(press("1", &approve_of_1));
            eventually(|| async { env.status_of("alice@gmail.com").await == "approved" }).await;
        });
    }

    #[test]
    fn a_button_cannot_overturn_a_decision_made_elsewhere() {
        scenario(|env| async move {
            env.owner().await;
            env.pair("1").await;
            env.register("Alice", "alice@gmail.com").await;
            let (approve, _) = env.tokens_for("1", 1).await;

            let (s, _) = env
                .admin("POST", "/admin/users/reject", Some(json!({"identifier": "alice@gmail.com"})))
                .await;
            assert_eq!(s, 200);

            env.fake.push_update(press("1", &approve));
            env.barrier("1").await;
            assert_eq!(env.status_of("alice@gmail.com").await, "rejected");
            assert_eq!(env.login("alice@gmail.com").await, (403, "ACCOUNT_REJECTED".into()));
            assert!(env.fake.sent().iter().any(|(c, m, _)| c == "1" && m.text.starts_with("Already decided")));
            assert_eq!(env.audit_count("user.approve", "Alice"), 0);
        });
    }

    #[test]
    fn simultaneous_decisions_from_chat_and_http_produce_exactly_one() {
        scenario(|env| async move {
            env.owner().await;
            env.pair("1").await;
            env.pair("2").await;

            // 5 轮：同一个 10 分钟窗口里超过 5 个申请会合并成一条摘要（没有按钮）。
            for i in 0..5 {
                let name = format!("Racer{i}");
                let email = format!("racer{i}@x.org");
                env.register(&name, &email).await;
                let (a1, _) = env.tokens_for("1", i + 1).await;
                let (a2, _) = env.tokens_for("2", i + 1).await;

                // 两位管理员点按钮，同一刻管理端点也批准。
                env.fake.push_update(press("1", &a1));
                env.fake.push_update(press("2", &a2));
                let (s, _) = env
                    .admin("POST", "/admin/users/approve", Some(json!({"identifier": email})))
                    .await;
                assert_eq!(s, 200);
                env.barrier("1").await;
                env.barrier("2").await;

                assert_eq!(env.status_of(&email).await, "approved");
                assert_eq!(env.audit_count("user.approve", &name), 1, "{name}：只能有一次真正的决定");
            }
        });
    }

    // ---- 验收 8：取消配对、配对限流 ----

    #[test]
    fn unpairing_over_http_revokes_at_once_and_drops_the_admins_tokens() {
        scenario(|env| async move {
            env.owner().await;
            let admin_id = env.pair("1").await;
            env.pair("2").await;
            env.register("Alice", "alice@gmail.com").await;
            let (approve, _) = env.tokens_for("1", 1).await;
            env.tokens_for("2", 1).await;

            let (s, body) = env.admin("DELETE", &format!("/admin/chat/admins/{admin_id}"), None).await;
            assert_eq!((s, body["data"]["label"].as_str()), (200, Some("Admin1")));
            let left: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM chat_action_tokens WHERE identity_id = ?")
                .bind(&admin_id)
                .fetch_one(&env.db)
                .await
                .unwrap();
            assert_eq!(left.0, 0, "取消配对要连同未用令牌一起删除");

            let sent = env.fake.sent().len();
            env.fake.push_update(press("1", &approve));
            env.fake.push_update(cmd("1", "pending", ""));
            env.barrier("2").await;
            assert_eq!(env.status_of("alice@gmail.com").await, "pending");
            assert_eq!(env.fake.sent().len(), sent + 1, "前管理员得不到任何回复（只有屏障的回复）");
        });
    }

    #[test]
    fn wrong_pairing_codes_are_limited_globally_not_only_per_sender() {
        scenario(|env| async move {
            env.owner().await;
            let (_, body) = env.admin("POST", "/admin/chat/pairing", None).await;
            let code = body["data"]["code"].as_str().unwrap().to_string();

            // 4 个不同发送者各试错 5 次 = 20 次，用满全局额度。
            for sender in ["10", "11", "12", "13"] {
                for _ in 0..5 {
                    env.fake.push_update(cmd(sender, "pair", "WRONGCODE"));
                }
            }
            // 第 5 个发送者带着正确的码也被拒（沉默）。
            env.fake.push_update(cmd("14", "pair", &code));
            // 没有已配对的管理员可做屏障；轮询循环每 5ms 一轮，半秒足够处理完这 21 条。
            tokio::time::sleep(Duration::from_millis(500)).await;
            assert!(env.admins().await.is_empty());
            assert!(env.fake.sent().is_empty(), "陌生人没有收到任何回复");
        });
    }

    // ---- 验收 9：聊天宕机 ----

    #[test]
    fn chat_outage_keeps_registrations_pending_and_the_outbox_delivers_later() {
        scenario(|env| async move {
            env.owner().await;
            env.pair("1").await;
            let before = env.fake.sent().len();
            env.fake.set_down(true);

            assert_eq!(env.register("Alice", "alice@gmail.com").await, 202);
            eventually(|| async {
                let attempts: Option<(i64,)> =
                    sqlx::query_as("SELECT attempts FROM chat_outbox WHERE kind = 'notify'")
                        .fetch_optional(&env.db)
                        .await
                        .unwrap();
                attempts.is_some_and(|(a,)| a >= 1)
            })
            .await;
            assert_eq!(env.status_of("alice@gmail.com").await, "pending", "宕机不会隐式批准");
            assert_eq!(env.login("alice@gmail.com").await.1, "ACCOUNT_PENDING");
            assert_eq!(env.fake.sent().len(), before);

            // 聊天恢复、退避到期：由发送循环补发（下一次注册会唤醒它）。
            env.fake.set_down(false);
            sqlx::query("UPDATE chat_outbox SET next_attempt_at = '2000-01-01T00:00:00+00:00' WHERE state = 'queued'")
                .execute(&env.db)
                .await
                .unwrap();
            assert_eq!(env.register("Bobby", "bobby@gmail.com").await, 202);
            eventually(|| async { env.buttoned("1").len() == 2 }).await;
            let texts: Vec<String> = env.buttoned("1").into_iter().map(|m| m.text).collect();
            assert!(texts.iter().any(|t| t.contains("a***@gmail.com")));
            assert!(texts.iter().any(|t| t.contains("b***@gmail.com")));
            assert_eq!(env.status_of("alice@gmail.com").await, "pending");
            assert_eq!(env.status_of("bobby@gmail.com").await, "pending");
        });
    }

    // ---- 验收 10：日志 ----

    #[test]
    fn no_secret_or_unmasked_identifier_reaches_the_logs() {
        scenario(|env| async move {
            env.owner().await;
            let (_, body) = env.admin("POST", "/admin/chat/pairing", None).await;
            let code = body["data"]["code"].as_str().unwrap().to_string();
            env.fake.push_update(cmd("1", "pair", &code));
            eventually(|| async { env.admins().await.len() == 1 }).await;
            env.fake.push_update(cmd("5", "pair", "BADBADBA"));

            env.register_from("Alice", "alice.private@gmail.com", Some("+886912345123"), REMOTE).await;
            let (approve, reject) = env.tokens_for("1", 1).await;
            env.login("alice.private@gmail.com").await;
            env.fake.push_update(press("1", &approve));
            eventually(|| async { env.status_of("alice.private@gmail.com").await == "approved" }).await;
            env.call("GET", "/admin/users", None, Some("bad-admin-guess"), LOCAL).await;
            env.barrier("1").await;

            let logs = env.logs.text();
            assert!(logs.contains("user.approve") && logs.contains("chat.pair"), "审计行应当存在：{logs}");
            for secret in [
                code.as_str(),
                approve.as_str(),
                reject.as_str(),
                "BADBADBA",
                PASSWORD,
                ADMIN_PW,
                "bad-admin-guess",
                "alice.private@gmail.com",
                "+886912345123",
                "owner@home.test",
            ] {
                assert!(!logs.contains(secret), "日志含有 {secret:?}");
            }
        });
    }
}
