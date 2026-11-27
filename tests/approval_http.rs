//! 账户审批的端到端测试（R-066，spec/admin_chat.md §8 第 1–6、10 条）：
//! 启动真实的 `cloud_home` 二进制（临时数据盘、一次性 JWT 密钥、空闲端口、临时日志目录），
//! 只通过 HTTP 驱动。聊天侧（FakeProvider）的验收见 `tests/chat_fake.rs`。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const ADMIN_PW: &str = "adm1n-secret-pw";
const JWT_SECRET: &str = "jwt-secret-for-approval-tests";
const PASSWORD: &str = "correct-horse-battery";
/// 公网地址：经 `X-Forwarded-For` 冒充远程调用者（服务端以 `TRUST_PROXY_HEADERS=true` 启动）。
const REMOTE: &str = "8.8.8.8";

struct Resp {
    status: u16,
    headers: Vec<(String, String)>,
    body: Value,
}

impl Resp {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// 错误响应里的机器可读错误码。
    fn code(&self) -> &str {
        self.body["error"]["code"].as_str().unwrap_or("")
    }
}

struct Server {
    child: Child,
    port: u16,
    dir: PathBuf,
}

impl Server {
    fn start(env: &[(&str, &str)]) -> Self {
        let dir = std::env::temp_dir().join(format!("ch-approval-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        Self::start_in(dir, env)
    }

    /// 在已有目录上（重新）启动：数据库随目录保留。
    fn start_in(dir: PathBuf, env: &[(&str, &str)]) -> Self {
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let stdout = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("stdout.log"))
            .unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cloud_home"));
        cmd.current_dir(&dir) // 避免读到仓库里的 .env
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("SERVER_HOST", "127.0.0.1")
            .env("SERVER_PORT", port.to_string())
            .env("JWT_SECRET", JWT_SECRET)
            .env("ADMIN_PASSWORD", ADMIN_PW)
            .env("DEV_DRIVE_PATH", dir.join("drive"))
            .env("VOLUMES_WATCH_PATH", dir.join("no-volumes"))
            .env("LOG_DIR", dir.join("logs"))
            .env("UPDATE_ROOT", dir.join("update"))
            .env("TRUST_PROXY_HEADERS", "true")
            .env("REGISTER_RATE_PER_IP_PER_HOUR", "1000")
            .env("PENDING_MAX", "1000")
            .env("CHAT_PROVIDER", "none")
            .stdout(Stdio::from(stdout.try_clone().unwrap()))
            .stderr(Stdio::from(stdout));
        for (k, v) in env {
            cmd.env(k, v);
        }
        let child = cmd.spawn().unwrap();
        let srv = Self { child, port, dir };
        srv.wait_ready();
        srv
    }

    fn wait_ready(&self) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("服务未在 30 秒内就绪：\n{}", self.log_text());
    }

    /// 停掉进程但保留目录，供重启或检查数据库。
    fn stop(mut self) -> PathBuf {
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::mem::take(&mut self.dir)
    }

    fn call(&self, method: &str, path: &str, headers: &[(&str, &str)], body: Option<&Value>) -> Resp {
        let mut s = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        let payload = body.map(Value::to_string).unwrap_or_default();
        let mut req = format!(
            "{method} /api/v1{path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n"
        );
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        if body.is_some() {
            req.push_str("Content-Type: application/json\r\n");
        }
        req.push_str(&format!("Content-Length: {}\r\n\r\n{payload}", payload.len()));
        s.write_all(req.as_bytes()).unwrap();
        let mut raw = Vec::new();
        s.read_to_end(&mut raw).unwrap();
        let text = String::from_utf8_lossy(&raw).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap();
        let mut lines = head.lines();
        let status = lines
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let headers = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
            .collect();
        Resp {
            status,
            headers,
            body: serde_json::from_str(body).unwrap_or(Value::Null),
        }
    }

    fn register(&self, name: &str, email: &str, from: Option<&str>) -> Resp {
        let xff: Vec<(&str, &str)> = from.map(|ip| ("X-Forwarded-For", ip)).into_iter().collect();
        self.call(
            "POST",
            "/auth/register",
            &xff,
            Some(&json!({"username": name, "password": PASSWORD, "email": email})),
        )
    }

    fn login(&self, email: &str, password: &str) -> Resp {
        self.call(
            "POST",
            "/auth/login",
            &[],
            Some(&json!({"identifier": email, "password": password})),
        )
    }

    fn admin(&self, method: &str, path: &str, body: Option<&Value>) -> Resp {
        self.call(method, path, &[("X-Admin-Password", ADMIN_PW)], body)
    }

    fn admin_post(&self, action: &str, email: &str) -> Resp {
        self.admin("POST", &format!("/admin/users/{action}"), Some(&json!({"identifier": email})))
    }

    /// 管理端列表里该邮箱对应账户的整行。
    fn user(&self, email: &str) -> Value {
        let list = self.admin("GET", "/admin/users", None);
        list.body["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|u| u["identities"].as_array().unwrap().iter().any(|i| i["identifier"] == email))
            .unwrap_or_else(|| panic!("找不到 {email}：{}", list.body))
            .clone()
    }

    fn me(&self, token: &str) -> Resp {
        self.call("GET", "/auth/me", &[("Authorization", &format!("Bearer {token}"))], None)
    }

    /// 连上这台服务正在用的 SQLite 文件，供测试直接改数据 / 造「不可能经 API 产生」的状态。
    async fn db(&self) -> sqlx::SqlitePool {
        sqlx::SqlitePool::connect(&format!("sqlite://{}", self.db_file().display()))
            .await
            .unwrap()
    }

    fn db_file(&self) -> PathBuf {
        db_file(&self.dir)
    }

    /// 标准输出与日志目录里的全部文本。
    fn log_text(&self) -> String {
        let mut text = std::fs::read_to_string(self.dir.join("stdout.log")).unwrap_or_default();
        collect_files(&self.dir.join("logs"), &mut text);
        text
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if !self.dir.as_os_str().is_empty() {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

fn db_file(dir: &Path) -> PathBuf {
    dir.join("drive/cloud_home_data/cloud_home.db")
}

fn collect_files(dir: &Path, out: &mut String) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_files(&p, out);
        } else if let Ok(t) = std::fs::read_to_string(&p) {
            out.push_str(&t);
        }
    }
}

/// 用服务端的 JWT 密钥给任意用户 id 签一个**有效**令牌：模拟「令牌是合法的，但账户不该被放行」。
fn forged_token(user_id: &str) -> String {
    let now = chrono::Utc::now().timestamp();
    jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &json!({"sub": user_id, "iat": now, "exp": now + 3600}),
        &jsonwebtoken::EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .unwrap()
}

/// 首个本机账户（引导批准），之后的申请都是 pending。
fn server_with_owner() -> Server {
    let srv = Server::start(&[]);
    assert_eq!(srv.register("Owner", "owner@home.test", None).status, 201);
    srv
}

// ---- 验收 1、2：引导规则 ----

#[test]
fn first_local_registration_is_approved_and_second_is_pending_without_token() {
    let srv = Server::start(&[]);

    let first = srv.register("Owner", "owner@home.test", None);
    assert_eq!(first.status, 201);
    assert!(first.body["data"]["token"].is_string());
    assert!(first.header("set-cookie").is_some_and(|c| c.starts_with("auth_token=")));
    assert_eq!(srv.login("owner@home.test", PASSWORD).status, 200);
    assert_eq!(srv.user("owner@home.test")["approval_decided_by"], "bootstrap");

    let second = srv.register("Guest", "guest@home.test", None);
    assert_eq!(second.status, 202);
    assert_eq!(second.body["data"]["status"], "pending_approval");
    assert!(second.header("set-cookie").is_none(), "pending 注册不得设 Cookie");
    assert!(!second.body.to_string().contains("token"), "pending 注册不得返回令牌");
}

#[test]
fn remote_first_registration_is_pending_and_does_not_bootstrap() {
    let srv = Server::start(&[]);

    let r = srv.register("Remote", "remote@far.test", Some(REMOTE));
    assert_eq!(r.status, 202);
    assert!(r.header("set-cookie").is_none());
    assert_eq!(srv.user("remote@far.test")["approval_status"], "pending");

    // 表非空后，本机来的下一个也不再享受引导。
    assert_eq!(srv.register("Local", "local@home.test", None).status, 202);
    assert_eq!(srv.user("local@home.test")["approval_status"], "pending");
}

#[test]
fn soft_deleted_rows_keep_the_table_non_empty_for_bootstrap() {
    let srv = Server::start(&[]);
    assert_eq!(srv.register("Owner", "owner@home.test", None).status, 201);
    assert_eq!(srv.admin_post("delete", "owner@home.test").status, 200);

    let again = srv.register("Intruder", "intruder@home.test", None);
    assert_eq!(again.status, 202, "只剩软删除行时也不是空表，不得再引导");
}

// ---- 验收 3：pending / rejected 拿不到也用不了令牌 ----

#[test]
fn pending_and_rejected_can_neither_obtain_nor_use_a_token() {
    let srv = server_with_owner();
    srv.register("Guest", "guest@home.test", None);
    let id = srv.user("guest@home.test")["id"].as_str().unwrap().to_string();

    // 密码对 → 403 ACCOUNT_PENDING，且没有令牌 / Cookie；密码错 → 401，不泄露状态。
    let pending = srv.login("guest@home.test", PASSWORD);
    assert_eq!((pending.status, pending.code()), (403, "ACCOUNT_PENDING"));
    assert!(pending.header("set-cookie").is_none());
    assert!(!pending.body.to_string().contains("token"));
    assert_eq!(srv.login("guest@home.test", "wrong-password").status, 401);

    // 令牌本身合法（签名与有效期都对），账户仍然被拒：每次请求都要查库。
    let token = forged_token(&id);
    assert_eq!(srv.me(&token).status, 401);
    let cookie = format!("auth_token={token}");
    assert_eq!(srv.call("GET", "/files", &[("Cookie", &cookie)], None).status, 401);

    assert_eq!(srv.admin_post("reject", "guest@home.test").status, 200);
    let rejected = srv.login("guest@home.test", PASSWORD);
    assert_eq!((rejected.status, rejected.code()), (403, "ACCOUNT_REJECTED"));
    assert!(rejected.header("set-cookie").is_none());
    assert_eq!(srv.login("guest@home.test", "wrong-password").status, 401);
    assert_eq!(srv.me(&token).status, 401);
}

#[test]
fn a_token_stops_working_at_the_next_request_after_the_account_is_deactivated() {
    let srv = server_with_owner();
    let owner = srv.login("owner@home.test", PASSWORD);
    let token = owner.body["data"]["token"].as_str().unwrap().to_string();
    assert_eq!(srv.me(&token).status, 200);

    assert_eq!(srv.admin_post("delete", "owner@home.test").status, 200);
    assert_eq!(srv.me(&token).status, 401);
    assert_eq!(srv.login("owner@home.test", PASSWORD).status, 401);
}

// ---- 验收 4：批准 ----

#[test]
fn approve_is_idempotent_and_reject_then_approve_is_allowed_and_audited() {
    let srv = server_with_owner();
    srv.register("Guest", "guest@home.test", None);

    // 管理端点受保护：无口令 401，远程调用 403。
    let body = json!({"identifier": "guest@home.test"});
    assert_eq!(srv.call("POST", "/admin/users/approve", &[], Some(&body)).status, 401);
    let remote = srv.call(
        "POST",
        "/admin/users/approve",
        &[("X-Admin-Password", ADMIN_PW), ("X-Forwarded-For", REMOTE)],
        Some(&body),
    );
    assert_eq!(remote.status, 403);
    assert_eq!(srv.user("guest@home.test")["approval_status"], "pending");

    assert_eq!(srv.admin_post("reject", "guest@home.test").status, 200);
    assert_eq!(srv.admin_post("reject", "guest@home.test").status, 200, "拒绝幂等");
    assert_eq!(srv.admin_post("approve", "guest@home.test").status, 200, "允许改判");
    assert_eq!(srv.admin_post("approve", "guest@home.test").status, 200, "批准幂等");
    assert_eq!(srv.admin_post("reject", "guest@home.test").status, 409, "已批准不能再拒绝");

    let ok = srv.login("guest@home.test", PASSWORD);
    assert_eq!(ok.status, 200);
    let token = ok.body["data"]["token"].as_str().unwrap();
    assert_eq!(srv.me(token).status, 200);
    let row = srv.user("guest@home.test");
    assert_eq!(
        (row["approval_status"].as_str(), row["approval_decided_by"].as_str(), row["is_active"].as_bool()),
        (Some("approved"), Some("admin-endpoint"), Some(true))
    );

    // 审计：每次真正的决定恰好一条，重复调用不再产生。
    std::thread::sleep(Duration::from_millis(300));
    let log = srv.log_text();
    // 文件日志是 JSON，一行一个事件；控制台是 pretty 格式，不会重复命中。
    let count = |ev: &str| {
        log.lines()
            .filter(|l| l.contains(&format!("\"event\":\"{ev}\"")) && l.contains("\"target_username\":\"Guest\""))
            .count()
    };
    assert_eq!((count("user.reject"), count("user.approve")), (1, 1));
}

// ---- 验收 5：还原不会放行未通过审批的账户 ----

#[tokio::test]
async fn restore_never_reactivates_a_pending_or_rejected_account() {
    let srv = server_with_owner();
    srv.register("Pending", "pending@home.test", None);
    srv.register("Rejected", "rejected@home.test", None);
    srv.admin_post("reject", "rejected@home.test");
    let db = srv.db().await;

    for email in ["pending@home.test", "rejected@home.test"] {
        let id = srv.user(email)["id"].as_str().unwrap().to_string();
        // 账户没被软删除时，还原是 409，状态不变；本来就停用的账户也不能被「删除」。
        assert_eq!(srv.admin_post("restore", email).status, 409);
        assert_eq!(srv.admin_post("delete", email).status, 409);

        // 即便行上被打了软删除标记（其他路径 / 手工改库），还原也只撤销删除、绝不放行。
        sqlx::query("UPDATE users SET deleted_at = ? WHERE id = ?")
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(&id)
            .execute(&db)
            .await
            .unwrap();
        assert_eq!(srv.admin_post("restore", email).status, 409, "{email}");
        let row = srv.user(email);
        assert_eq!(row["is_active"], false, "{email}");
        assert_ne!(row["approval_status"], "approved", "{email}");
        assert!(row["deleted_at"].is_string(), "{email}");
        assert_eq!(srv.me(&forged_token(&id)).status, 401);
        assert_ne!(srv.login(email, PASSWORD).status, 200);
    }

    // 对照：已批准账户删除后还原是可以的。
    srv.register("Fine", "fine@home.test", None);
    srv.admin_post("approve", "fine@home.test");
    assert_eq!(srv.admin_post("delete", "fine@home.test").status, 200);
    assert_eq!(srv.login("fine@home.test", PASSWORD).status, 401);
    assert_eq!(srv.admin_post("restore", "fine@home.test").status, 200);
    assert_eq!(srv.login("fine@home.test", PASSWORD).status, 200);
    db.close().await;
}

// ---- 验收 6：注册模式、限流、重新申请、过期 ----

#[test]
fn closed_mode_refuses_every_registration() {
    let srv = Server::start(&[("REGISTRATION_MODE", "closed")]);
    let r = srv.register("Owner", "owner@home.test", None);
    assert_eq!((r.status, r.code()), (403, "REGISTRATION_CLOSED"));
    assert_eq!(srv.admin("GET", "/admin/users", None).body["data"]["items"], json!([]));
}

#[test]
fn pending_max_and_the_per_ip_rate_limit_return_429() {
    let srv = Server::start(&[("PENDING_MAX", "2"), ("REGISTER_RATE_PER_IP_PER_HOUR", "1000")]);
    assert_eq!(srv.register("Owner", "owner@home.test", None).status, 201);
    assert_eq!(srv.register("Alice", "a@home.test", None).status, 202);
    assert_eq!(srv.register("Bobby", "b@home.test", None).status, 202);
    let full = srv.register("Carol", "c@home.test", None);
    assert_eq!((full.status, full.code()), (429, "REGISTRATION_BUSY"));
    // 批准一个，队列腾出位置。
    srv.admin_post("approve", "a@home.test");
    assert_eq!(srv.register("Carol", "c@home.test", None).status, 202);

    let limited = Server::start(&[("REGISTER_RATE_PER_IP_PER_HOUR", "2")]);
    assert_eq!(limited.register("Olive", "o@home.test", Some("1.1.1.1")).status, 202);
    assert_eq!(limited.register("Peter", "p@home.test", Some("1.1.1.1")).status, 202);
    let over = limited.register("Quinn", "q@home.test", Some("1.1.1.1"));
    assert_eq!((over.status, over.code()), (429, "REGISTRATION_BUSY"));
    // 限流按来源 IP：另一个来源不受影响。
    assert_eq!(limited.register("Ruth!", "r@home.test", Some("2.2.2.2")).status, 202);
}

#[test]
fn rejected_applicant_registers_again_at_once_and_gets_no_token() {
    let srv = server_with_owner();
    srv.register("Guest", "guest@home.test", None);
    srv.admin_post("reject", "guest@home.test");
    assert_eq!(srv.login("guest@home.test", PASSWORD).code(), "ACCOUNT_REJECTED");

    let again = srv.register("Guest", "guest@home.test", None);
    assert_eq!(again.status, 202);
    assert!(again.header("set-cookie").is_none());
    assert!(!again.body.to_string().contains("token"));
    assert_eq!(srv.user("guest@home.test")["approval_status"], "pending");
    assert_eq!(srv.login("guest@home.test", PASSWORD).code(), "ACCOUNT_PENDING");
}

#[tokio::test]
async fn expiry_task_rejects_stale_pending_accounts_and_leaves_fresh_ones() {
    let srv = server_with_owner();
    srv.register("Old", "old@home.test", None);
    srv.register("New", "new@home.test", None);
    let dir = srv.stop();

    // 把 Old 的申请时间改到 30 天前，然后带着 PENDING_EXPIRE_DAYS=14 重启：过期任务启动即跑一轮。
    let db = sqlx::SqlitePool::connect(&format!("sqlite://{}", db_file(&dir).display()))
        .await
        .unwrap();
    let old = (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339();
    sqlx::query(
        "UPDATE users SET created_at = ? WHERE id IN (SELECT user_id FROM user_identities WHERE identifier = 'old@home.test')",
    )
    .bind(&old)
    .execute(&db)
    .await
    .unwrap();
    db.close().await;

    let srv = Server::start_in(dir, &[("PENDING_EXPIRE_DAYS", "14")]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while srv.user("old@home.test")["approval_status"] != "rejected" && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    let old = srv.user("old@home.test");
    assert_eq!(old["approval_status"], "rejected");
    assert_eq!(old["approval_decided_by"], "expiry");
    assert_eq!(srv.user("new@home.test")["approval_status"], "pending");
    assert_eq!(srv.login("old@home.test", PASSWORD).code(), "ACCOUNT_REJECTED");
}

// ---- 验收 10：日志里没有密钥，也没有未打码的标识 ----

#[test]
fn logs_hold_no_secret_and_no_unmasked_identifier() {
    let srv = server_with_owner();
    srv.register("Guest", "guest.private@home.test", Some(REMOTE));
    let owner = srv.login("owner@home.test", PASSWORD);
    let token = owner.body["data"]["token"].as_str().unwrap().to_string();
    srv.login("guest.private@home.test", PASSWORD); // 403 pending
    srv.login("nobody.here@home.test", "wrong-password-1"); // 401 未知账户
    srv.login("owner@home.test", "wrong-password-2"); // 401 密码错
    srv.admin_post("approve", "guest.private@home.test");
    srv.call("GET", "/admin/users", &[("X-Admin-Password", "bad-admin-guess")], None);
    srv.me(&token);
    std::thread::sleep(Duration::from_millis(500));

    let log = srv.log_text();
    assert!(log.contains("user.approve"), "审计行应当存在：{log}");
    for secret in [
        PASSWORD,
        "wrong-password-1",
        "wrong-password-2",
        ADMIN_PW,
        "bad-admin-guess",
        JWT_SECRET,
        token.as_str(),
    ] {
        assert!(!log.contains(secret), "日志含有机密 {secret:?}");
    }
    for ident in ["guest.private@home.test", "nobody.here@home.test", "owner@home.test"] {
        assert!(!log.contains(ident), "日志含有未打码标识 {ident:?}");
    }
}
