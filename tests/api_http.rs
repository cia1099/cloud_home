//! 后端 HTTP 集成测试（R-030）：启动真实的 `cloud_home` 二进制（临时数据盘、一次性 JWT 密钥、
//! 空闲端口、临时日志目录），只通过 HTTP 驱动。
//!
//! 第一优先级是账户隔离：账户 A 永远不能列出、读取、下载、缩略图、改名、移动、删除、分享、
//! 搜索、还原账户 B 的文件 / 资料夹 / 回收站 / 分享，哪怕拿到了对方的 id。
//! 站内定向分享的收件人访问与审批流程另见 `share_recipients_http.rs`、`approval_http.rs`。

use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const ADMIN_PW: &str = "adm1n-secret-pw";
const PASSWORD: &str = "correct-horse-battery";
const BOUNDARY: &str = "XBOUNDARYX";
/// 公网地址：经 `X-Forwarded-For` 冒充远程调用者。
const REMOTE: &str = "8.8.8.8";

struct Resp {
    status: u16,
    headers: Vec<(String, String)>,
    raw: Vec<u8>,
    body: Value,
}

impl Resp {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn data(&self) -> &Value {
        &self.body["data"]
    }

    fn code(&self) -> &str {
        self.body["error"]["code"].as_str().unwrap_or("")
    }

    fn items(&self) -> &Vec<Value> {
        self.body["data"]["items"]
            .as_array()
            .unwrap_or_else(|| panic!("无 items：{}", self.body))
    }

    fn names(&self) -> Vec<&str> {
        self.items()
            .iter()
            .map(|i| i["name"].as_str().unwrap())
            .collect()
    }
}

/// 一个已登录的测试账户。
struct Acct {
    id: String,
    token: String,
    email: String,
}

struct Server {
    child: Child,
    port: u16,
    dir: PathBuf,
}

impl Server {
    fn start() -> Self {
        Self::start_with(&[])
    }

    fn start_with(env: &[(&str, &str)]) -> Self {
        let dir = std::env::temp_dir().join(format!("ch-api-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        Self::start_in(dir, env)
    }

    /// 在已有目录上（重新）启动：数据库与文件随目录保留。
    /// 空闲端口是「先探测、后绑定」选出来的，并行的测试进程可能抢先占用；
    /// 服务没能在自己的端口上起来（进程退出）就换一个端口重试，绝不把请求发给别人的服务。
    fn start_in(dir: PathBuf, env: &[(&str, &str)]) -> Self {
        for _ in 0..5 {
            if let Some(srv) = Self::try_start(&dir, env) {
                return srv;
            }
        }
        panic!(
            "连续 5 次都没能启动服务：\n{}",
            std::fs::read_to_string(dir.join("stdout.log")).unwrap_or_default()
        );
    }

    fn try_start(dir: &Path, env: &[(&str, &str)]) -> Option<Self> {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let out = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("stdout.log"))
            .unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cloud_home"));
        cmd.current_dir(dir) // 避免读到仓库里的 .env
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("SERVER_HOST", "127.0.0.1")
            .env("SERVER_PORT", port.to_string())
            .env("JWT_SECRET", "jwt-secret-for-api-tests")
            .env("ADMIN_PASSWORD", ADMIN_PW)
            .env("DEV_DRIVE_PATH", dir.join("drive"))
            .env("VOLUMES_WATCH_PATH", dir.join("no-volumes"))
            .env("LOG_DIR", dir.join("logs"))
            .env("UPDATE_ROOT", dir.join("update"))
            .env("TRUST_PROXY_HEADERS", "true")
            .env("REGISTRATION_MODE", "open")
            .env("CHAT_PROVIDER", "none")
            .stdout(Stdio::from(out.try_clone().unwrap()))
            .stderr(Stdio::from(out));
        for (k, v) in env {
            cmd.env(k, v);
        }
        let child = cmd.spawn().unwrap();
        let mut srv = Self {
            child,
            port,
            dir: dir.to_path_buf(),
        };
        let deadline = Instant::now() + Duration::from_secs(30);
        // 就绪 = 这个子进程自己打印了「已启动」且监听着我们选的端口（而不是别人的服务）。
        let started = format!(":{port}");
        loop {
            if srv.child.try_wait().unwrap().is_some() {
                srv.dir = PathBuf::new(); // 保留目录，供下一次尝试
                return None;
            }
            let log = std::fs::read_to_string(dir.join("stdout.log")).unwrap_or_default();
            if log
                .lines()
                .any(|l| l.contains("已启动") && l.contains(&started))
            {
                return Some(srv);
            }
            assert!(
                Instant::now() < deadline,
                "服务未在 30 秒内就绪：\n{}",
                srv.log_text()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// 停掉进程但保留目录，供重启。
    fn stop(mut self) -> PathBuf {
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::mem::take(&mut self.dir)
    }

    fn connect(&self) -> TcpStream {
        let s = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
        s
    }

    fn send(&self, method: &str, path: &str, headers: &[(String, String)], body: &[u8]) -> Resp {
        let mut s = self.connect();
        let mut req =
            format!("{method} /api/v1{path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
        s.write_all(req.as_bytes()).unwrap();
        s.write_all(body).unwrap();
        let mut raw = Vec::new();
        s.read_to_end(&mut raw)
            .unwrap_or_else(|e| panic!("{method} {path}: {e}"));
        parse_response(&raw)
    }

    fn call_h(
        &self,
        who: Option<&Acct>,
        method: &str,
        path: &str,
        body: Option<&Value>,
        extra: &[(&str, &str)],
    ) -> Resp {
        let mut h: Vec<(String, String)> = extra
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        if let Some(a) = who {
            h.push(("Authorization".into(), format!("Bearer {}", a.token)));
        }
        if body.is_some() {
            h.push(("Content-Type".into(), "application/json".into()));
        }
        let payload = body.map(Value::to_string).unwrap_or_default();
        self.send(method, path, &h, payload.as_bytes())
    }

    fn call(&self, who: Option<&Acct>, method: &str, path: &str, body: Option<&Value>) -> Resp {
        self.call_h(who, method, path, body, &[])
    }

    fn get(&self, who: &Acct, path: &str) -> Resp {
        self.call(Some(who), "GET", path, None)
    }

    fn admin(&self, path: &str, body: &Value) -> Resp {
        self.call_h(
            None,
            "POST",
            path,
            Some(body),
            &[("X-Admin-Password", ADMIN_PW)],
        )
    }

    /// 注册（open 模式，201 + 令牌）。
    fn signup(&self, name: &str) -> Acct {
        let email = format!("{name}@home.test");
        let r = self.call(
            None,
            "POST",
            "/auth/register",
            Some(&json!({"username": name, "password": PASSWORD, "email": email})),
        );
        assert_eq!(r.status, 201, "register {name}: {}", r.body);
        Acct {
            id: r.data()["user"]["id"].as_str().unwrap().to_string(),
            token: r.data()["token"].as_str().unwrap().to_string(),
            email,
        }
    }

    fn upload_raw(&self, who: &Acct, parent: Option<&str>, name: &str, content: &[u8]) -> Resp {
        let mut body = Vec::new();
        if let Some(p) = parent {
            body.extend(
                format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"parent_id\"\r\n\r\n{p}\r\n")
                    .as_bytes(),
            );
        }
        body.extend(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend(content);
        body.extend(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
        self.send(
            "POST",
            "/files/upload",
            &[
                ("Authorization".into(), format!("Bearer {}", who.token)),
                (
                    "Content-Type".into(),
                    format!("multipart/form-data; boundary={BOUNDARY}"),
                ),
            ],
            &body,
        )
    }

    /// 上传并返回文件 id。
    fn upload(&self, who: &Acct, parent: Option<&str>, name: &str, content: &[u8]) -> String {
        let r = self.upload_raw(who, parent, name, content);
        assert_eq!(r.status, 201, "upload {name}: {}", r.body);
        r.data()["id"].as_str().unwrap().to_string()
    }

    fn folder(&self, who: &Acct, parent: Option<&str>, name: &str) -> String {
        let r = self.call(
            Some(who),
            "POST",
            "/folders",
            Some(&json!({"name": name, "parent_id": parent})),
        );
        assert_eq!(r.status, 201, "folder {name}: {}", r.body);
        r.data()["id"].as_str().unwrap().to_string()
    }

    fn delete_file(&self, who: &Acct, id: &str) {
        let r = self.call(Some(who), "DELETE", &format!("/files/{id}"), None);
        assert_eq!(r.status, 200, "delete: {}", r.body);
    }

    /// 回收站记录 id（按文件 id 查）。
    fn trash_id(&self, who: &Acct, file_id: &str) -> String {
        let r = self.get(who, "/trash");
        r.items()
            .iter()
            .find(|i| i["file_id"] == file_id)
            .unwrap_or_else(|| panic!("回收站里没有 {file_id}：{}", r.body))["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn share(&self, who: &Acct, file_id: &str, extra: Value) -> Value {
        let mut body = json!({"file_id": file_id});
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let r = self.call(Some(who), "POST", "/shares", Some(&body));
        assert_eq!(r.status, 201, "share: {}", r.body);
        r.data().clone()
    }

    fn usage(&self, who: &Acct) -> Value {
        self.get(who, "/usage").data().clone()
    }

    fn list(&self, who: &Acct, parent: Option<&str>) -> Resp {
        let path = parent.map_or("/files".to_string(), |p| format!("/files?parent_id={p}"));
        self.get(who, &path)
    }

    /// 服务端日志里该邮箱最近一次签发的验证码（开发期投递写在 `otp.issued` 审计行里）。
    fn otp_code(&self, email: &str) -> String {
        let masked = format!("{}***@{}", &email[..2], email.split_once('@').unwrap().1);
        let text = self.log_text();
        let line = text
            .lines()
            .rfind(|l| l.contains("otp.issued") && l.contains(&masked))
            .unwrap_or_else(|| panic!("日志里没有 {masked} 的验证码"));
        let v: Value = serde_json::from_str(line).unwrap();
        let fields = v.get("fields").unwrap_or(&v);
        fields["code"].as_str().unwrap().to_string()
    }

    /// 磁盘上全部用户文件（不含缩略图缓存）。
    fn disk_files(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        walk(&self.dir.join("drive/cloud_home_data/users"), &mut out);
        out.retain(|p| !p.components().any(|c| c.as_os_str() == ".thumbnails"));
        out
    }

    async fn db(&self) -> sqlx::SqlitePool {
        sqlx::SqlitePool::connect(&format!(
            "sqlite://{}",
            self.dir
                .join("drive/cloud_home_data/cloud_home.db")
                .display()
        ))
        .await
        .unwrap()
    }

    /// 直接改库：造「不可能在测试时间内经 API 产生」的状态（如已过期的回收站条目）。
    fn expire_trash(&self, trash_id: &str) {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let pool = self.db().await;
            sqlx::query("UPDATE trash SET expires_at = '2000-01-01T00:00:00+00:00' WHERE id = ?")
                .bind(trash_id)
                .execute(&pool)
                .await
                .unwrap();
        });
    }

    fn log_text(&self) -> String {
        let mut text = std::fs::read_to_string(self.dir.join("stdout.log")).unwrap_or_default();
        let mut files = Vec::new();
        walk(&self.dir.join("logs"), &mut files);
        for f in files {
            text.push_str(&std::fs::read_to_string(f).unwrap_or_default());
        }
        text
    }

    /// 以 Bearer 令牌订阅 SSE。
    fn events(&self, who: &Acct) -> Sse {
        let mut s = self.connect();
        let req = format!(
            "GET /api/v1/events/stream HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nAccept: text/event-stream\r\n\r\n",
            who.token
        );
        s.write_all(req.as_bytes()).unwrap();
        let mut reader = BufReader::new(s);
        let mut status = String::new();
        reader.read_line(&mut status).unwrap();
        assert!(status.contains("200"), "SSE 状态：{status}");
        loop {
            let mut l = String::new();
            reader.read_line(&mut l).unwrap();
            if l == "\r\n" {
                break;
            }
        }
        Sse {
            reader,
            line: String::new(),
            event: None,
            data: String::new(),
        }
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

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn parse_response(raw: &[u8]) -> Resp {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("响应没有头部结束符");
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let mut body = raw[split + 4..].to_vec();
    let mut lines = head.lines();
    let status = lines
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    if headers
        .iter()
        .any(|(k, v)| k == "transfer-encoding" && v == "chunked")
    {
        body = dechunk(&body);
    }
    Resp {
        status,
        headers,
        body: serde_json::from_slice(&body).unwrap_or(Value::Null),
        raw: body,
    }
}

fn dechunk(mut rest: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let eol = rest
            .windows(2)
            .position(|w| w == b"\r\n")
            .expect("chunk 头不完整");
        let size =
            usize::from_str_radix(std::str::from_utf8(&rest[..eol]).unwrap().trim(), 16).unwrap();
        if size == 0 {
            return out;
        }
        out.extend_from_slice(&rest[eol + 2..eol + 2 + size]);
        rest = &rest[eol + 2 + size + 2..];
    }
}

/// 已建立的 SSE 连接。
struct Sse {
    reader: BufReader<TcpStream>,
    line: String,
    event: Option<String>,
    data: String,
}

impl Sse {
    /// 在 `wait` 内读下一条事件；超时返回 None。
    fn next(&mut self, wait: Duration) -> Option<(String, Value)> {
        self.reader.get_ref().set_read_timeout(Some(wait)).unwrap();
        loop {
            match self.reader.read_line(&mut self.line) {
                Ok(0) => return None,
                Ok(_) if !self.line.ends_with('\n') => continue,
                Ok(_) => {}
                Err(_) => return None,
            }
            let l = std::mem::take(&mut self.line);
            let l = l.trim_end_matches(['\r', '\n']);
            if let Some(e) = l.strip_prefix("event:") {
                self.event = Some(e.trim().to_string());
            } else if let Some(d) = l.strip_prefix("data:") {
                self.data.push_str(d.trim());
            } else if l.is_empty()
                && let Some(e) = self.event.take()
            {
                let data =
                    serde_json::from_str(&std::mem::take(&mut self.data)).unwrap_or(Value::Null);
                return Some((e, data));
            }
        }
    }

    /// 读事件直到遇到指定类型（并满足条件），期间忽略其它事件。
    fn wait_for(&mut self, name: &str, ok: impl Fn(&Value) -> bool) -> Option<Value> {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            if let Some((e, d)) = self.next(Duration::from_secs(2))
                && e == name
                && ok(&d)
            {
                return Some(d);
            }
        }
        None
    }

    /// 在 `wait` 内是否出现过指定类型的事件。
    fn sees(&mut self, name: &str, wait: Duration) -> bool {
        let deadline = Instant::now() + wait;
        while Instant::now() < deadline {
            if let Some((e, _)) = self.next(Duration::from_millis(300))
                && e == name
            {
                return true;
            }
        }
        false
    }
}

fn png_bytes() -> Vec<u8> {
    let img = image::RgbImage::from_pixel(300, 200, image::Rgb([200, 30, 30]));
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn two() -> (Server, Acct, Acct) {
    let srv = Server::start();
    let alice = srv.signup("alice");
    let bob = srv.signup("bob");
    (srv, alice, bob)
}

// =====================================================================
// 优先级 1：账户隔离
// =====================================================================

/// 两个账户各有文件、资料夹（含子文件）、回收站条目、公开分享。B 拿 A 的 id 访问每一个
/// 文件 / 资料夹端点：一律 404，且与「根本不存在的 id」的响应完全一致（不泄露存在性）。
#[test]
fn user_b_cannot_touch_user_a_files_and_folders_even_with_guessed_ids() {
    let (srv, alice, bob) = two();
    let folder = srv.folder(&alice, None, "AlbumA");
    let inside = srv.upload(&alice, Some(&folder), "inside.txt", b"inside-secret");
    let file = srv.upload(&alice, None, "secret.txt", b"alice-secret");
    let image = srv.upload(&alice, None, "pic.png", &png_bytes());
    let mine = srv.upload(&bob, None, "mine.txt", b"bob-data");
    let mine_folder = srv.folder(&bob, None, "BobDir");
    let ghost = uuid::Uuid::new_v4().to_string();

    for id in [&file, &folder, &inside, &image] {
        for (method, tail, body) in [
            ("GET", "", None),
            ("GET", "/download", None),
            ("GET", "/raw", None),
            ("GET", "/thumbnail", None),
            ("GET", "/thumbnail?size=64", None),
            ("PATCH", "", Some(json!({"name": "hijacked"}))),
            (
                "POST",
                "/move",
                Some(json!({"target_parent_id": mine_folder})),
            ),
            ("DELETE", "", None),
        ] {
            let path = format!("/files/{id}{tail}");
            let r = srv.call(Some(&bob), method, &path, body.as_ref());
            let g = srv.call(
                Some(&bob),
                method,
                &format!("/files/{ghost}{tail}"),
                body.as_ref(),
            );
            assert_eq!(r.status, 404, "{method} {path}: {}", r.body);
            assert_eq!(
                r.body, g.body,
                "{method} {path}：A 的 id 与不存在的 id 响应不同，泄露存在性"
            );
        }
    }

    // 目录列表：B 不能列 A 的资料夹，也看不到 A 的根目录内容。
    assert_eq!(srv.list(&bob, Some(&folder)).status, 404);
    assert_eq!(srv.list(&bob, None).names(), ["BobDir", "mine.txt"]);

    // 往 A 的资料夹里建东西、传东西、把自己的东西移进去：全部 404，A 的资料夹保持原样。
    assert_eq!(
        srv.call(
            Some(&bob),
            "POST",
            "/folders",
            Some(&json!({"name": "x", "parent_id": folder}))
        )
        .status,
        404
    );
    assert_eq!(
        srv.upload_raw(&bob, Some(&folder), "plant.txt", b"x")
            .status,
        404
    );
    assert_eq!(
        srv.call(
            Some(&bob),
            "POST",
            &format!("/files/{mine}/move"),
            Some(&json!({"target_parent_id": folder}))
        )
        .status,
        404
    );
    assert_eq!(srv.list(&alice, Some(&folder)).names(), ["inside.txt"]);
    assert_eq!(
        srv.disk_files().len(),
        4,
        "B 的失败上传不应在磁盘上留下文件"
    );

    // 批量下载：任一 id 不属于自己就整体 404，不泄露任何内容；混入自己的 id 也一样。
    for ids in [
        json!([file]),
        json!([folder]),
        json!([mine, file]),
        json!([ghost]),
    ] {
        for format in ["zip", "tar.gz"] {
            let r = srv.call(
                Some(&bob),
                "POST",
                "/files/download",
                Some(&json!({"ids": ids, "format": format})),
            );
            assert_eq!(r.status, 404, "批量下载 {ids} {format}");
            assert!(!String::from_utf8_lossy(&r.raw).contains("alice-secret"));
        }
    }

    // A 的数据完好无损。
    let r = srv.get(&alice, &format!("/files/{file}/download"));
    assert_eq!(
        (r.status, r.raw.as_slice()),
        (200, b"alice-secret".as_slice())
    );
    assert_eq!(
        srv.get(&alice, &format!("/files/{inside}")).data()["name"],
        "inside.txt"
    );
}

#[test]
fn search_never_returns_other_accounts_files_or_trash() {
    let (srv, alice, bob) = two();
    let folder = srv.folder(&alice, None, "invoice-folder");
    srv.upload(&alice, Some(&folder), "invoice-2026.pdf", b"a");
    let trashed = srv.upload(&alice, None, "invoice-old.pdf", b"a");
    srv.delete_file(&alice, &trashed);
    srv.upload(&bob, None, "invoice-bob.pdf", b"b");

    let r = srv.get(&bob, "/search?q=invoice");
    assert_eq!(r.status, 200);
    assert_eq!(r.names(), ["invoice-bob.pdf"]);
    assert_eq!(r.data()["pagination"]["total"], 1);
    // LIKE 通配符不能拿来越过账户边界；类型过滤同理。
    for q in ["%25", "_", "%25invoice%25", "pdf", "folder"] {
        let r = srv.get(&bob, &format!("/search?q={q}"));
        assert_eq!(r.status, 200, "q={q}");
        assert!(
            r.names().iter().all(|n| *n == "invoice-bob.pdf"),
            "q={q} 泄露：{}",
            r.body
        );
    }
    assert!(
        srv.get(&bob, "/search?q=invoice&type=folder")
            .items()
            .is_empty()
    );
    // A 自己能搜到文件与资料夹，但搜不到回收站里的。
    let own = srv.get(&alice, "/search?q=invoice");
    let mut names = own.names();
    names.sort();
    assert_eq!(names, ["invoice-2026.pdf", "invoice-folder"]);
    assert_eq!(
        own.items()
            .iter()
            .find(|i| i["name"] == "invoice-2026.pdf")
            .unwrap()["parent_path"],
        "invoice-folder"
    );
    // 参数校验。
    assert_eq!(srv.get(&alice, "/search?q=").status, 400);
    assert_eq!(
        srv.get(&alice, &format!("/search?q={}", "a".repeat(101)))
            .status,
        400
    );
    assert_eq!(srv.get(&alice, "/search?q=a&type=bogus").status, 400);
    assert_eq!(srv.call(None, "GET", "/search?q=invoice", None).status, 401);
}

#[test]
fn user_b_cannot_see_restore_or_purge_user_a_trash() {
    let (srv, alice, bob) = two();
    let a1 = srv.upload(&alice, None, "a1.txt", b"first-secret");
    let a2 = srv.upload(&alice, None, "a2.txt", b"second-secret");
    srv.delete_file(&alice, &a1);
    srv.delete_file(&alice, &a2);
    let t1 = srv.trash_id(&alice, &a1);
    let b1 = srv.upload(&bob, None, "b1.txt", b"bob");
    srv.delete_file(&bob, &b1);

    // B 的回收站只有 B 自己的。
    assert_eq!(srv.get(&bob, "/trash").names(), ["b1.txt"]);
    // 用 trash id 或文件 id 去还原 / 永久删除 A 的条目：404。
    for id in [&t1, &a1] {
        assert_eq!(
            srv.call(Some(&bob), "POST", &format!("/trash/{id}/restore"), None)
                .status,
            404
        );
        assert_eq!(
            srv.call(Some(&bob), "DELETE", &format!("/trash/{id}"), None)
                .status,
            404
        );
    }
    // 回收站里的文件也不能通过文件端点读取（对 A 自己也一样：已在回收站就是不存在）。
    assert_eq!(srv.get(&bob, &format!("/files/{a1}/download")).status, 404);
    assert_eq!(srv.get(&alice, &format!("/files/{a1}")).status, 404);

    // B 清空回收站只清自己的。
    let r = srv.call(Some(&bob), "DELETE", "/trash", None);
    assert_eq!(r.status, 200);
    assert_eq!(r.data()["deleted_count"], 1);
    assert_eq!(srv.get(&alice, "/trash").items().len(), 2);
    assert_eq!(srv.disk_files().len(), 2, "A 的两个文件仍在磁盘上");

    // A 的条目完好，还原后内容不变。
    let r = srv.call(Some(&alice), "POST", &format!("/trash/{t1}/restore"), None);
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(
        srv.get(&alice, &format!("/files/{a1}/download")).raw,
        b"first-secret"
    );
}

#[test]
fn user_b_cannot_see_use_or_revoke_user_a_shares() {
    let (srv, alice, bob) = two();
    let file = srv.upload(&alice, None, "shared.txt", b"share-me");
    let other = srv.upload(&bob, None, "bobs.txt", b"bobs");
    let share = srv.share(&alice, &file, json!({}));
    let sid = share["id"].as_str().unwrap();
    let token = share["token"].as_str().unwrap();

    // B 不能给 A 的文件建分享（含定向分享给自己），列表里也看不到 A 的分享。
    for extra in [json!({}), json!({"recipient_ids": [bob.id]})] {
        let mut body = json!({"file_id": file});
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert_eq!(
            srv.call(Some(&bob), "POST", "/shares", Some(&body)).status,
            404
        );
    }
    assert!(srv.get(&bob, "/shares").items().is_empty());
    assert_eq!(srv.get(&alice, "/shares").items().len(), 1);
    // B 不能撤销、改收件人、标记已读 A 的分享，分享仍然有效。
    assert_eq!(
        srv.call(Some(&bob), "DELETE", &format!("/shares/{sid}"), None)
            .status,
        404
    );
    assert_eq!(
        srv.call(
            Some(&bob),
            "PATCH",
            &format!("/shares/{sid}"),
            Some(&json!({"recipient_ids": [bob.id]}))
        )
        .status,
        404
    );
    assert_eq!(
        srv.call(Some(&bob), "POST", &format!("/shares/{sid}/seen"), None)
            .status,
        404
    );
    // B 不能以收件人身份读 A 的公开分享（B 不是收件人）。
    assert_eq!(
        srv.get(&bob, &format!("/shares/received/{sid}/files/{file}"))
            .status,
        404
    );
    assert!(srv.get(&bob, "/shares/received").items().is_empty());
    assert_eq!(
        srv.call(
            None,
            "GET",
            &format!("/public/shares/{token}/download"),
            None
        )
        .raw,
        b"share-me"
    );

    // B 自己的分享不会暴露 A 的文件：公开 token 只指向 B 的文件。
    let bshare = srv.share(&bob, &other, json!({}));
    let r = srv.call(
        None,
        "GET",
        &format!("/public/shares/{}", bshare["token"].as_str().unwrap()),
        None,
    );
    assert_eq!(r.data()["name"], "bobs.txt");
    // 猜 token：不存在 / 截断 / 篡改一律 404。
    for bad in [&token[..token.len() - 1], "AAAA", &format!("{token}x")] {
        assert_eq!(
            srv.call(None, "GET", &format!("/public/shares/{bad}"), None)
                .status,
            404
        );
        assert_eq!(
            srv.call(None, "GET", &format!("/public/shares/{bad}/download"), None)
                .status,
            404
        );
    }
}

#[test]
fn usage_numbers_are_per_account_and_sse_never_crosses_accounts() {
    let (srv, alice, bob) = two();
    let mut bob_stream = srv.events(&bob);
    let mut alice_stream = srv.events(&alice);
    // 首帧：各自的 usage。
    assert_eq!(bob_stream.next(Duration::from_secs(5)).unwrap().0, "usage");
    assert_eq!(
        alice_stream.next(Duration::from_secs(5)).unwrap().0,
        "usage"
    );

    let file = srv.upload(&alice, None, "big.bin", &[7u8; 5000]);
    // A 看到 files:created 与新的 usage；B 什么都收不到。
    let created = alice_stream
        .wait_for("files", |d| d["kind"] == "created")
        .expect("A 没收到 created");
    assert_eq!(created["file"]["id"], file);
    let usage = alice_stream
        .wait_for("usage", |d| d["used_bytes"] == 5000)
        .expect("A 没收到新的 usage");
    assert_eq!(usage["file_count"], 1);
    assert!(
        !bob_stream.sees("files", Duration::from_secs(2)),
        "A 的文件事件泄露给了 B"
    );
    assert!(
        !bob_stream.sees("usage", Duration::from_millis(500)),
        "A 的占用泄露给了 B"
    );

    // /usage 只统计本人：A 5000 字节，B 为 0；删除后转入回收站字节数。
    assert_eq!(srv.usage(&alice)["used_bytes"], 5000);
    assert_eq!(srv.usage(&bob)["used_bytes"], 0);
    assert_eq!(srv.usage(&bob)["file_count"], 0);
    srv.delete_file(&alice, &file);
    assert!(
        alice_stream
            .wait_for("files", |d| d["kind"] == "deleted")
            .is_some()
    );
    let u = srv.usage(&alice);
    assert_eq!(
        (
            u["used_bytes"].as_i64(),
            u["trashed_bytes"].as_i64(),
            u["file_count"].as_i64()
        ),
        (Some(0), Some(5000), Some(0))
    );
    assert_eq!(srv.usage(&bob)["trashed_bytes"], 0);
    // 整盘占比列表按设计对所有登录用户可见（spec：任意已登录用户），但只含聚合数字。
    let r = srv.get(&bob, "/usage/breakdown");
    assert_eq!(r.status, 200);
    let a_row = r.items().iter().find(|i| i["user_id"] == alice.id).unwrap();
    assert_eq!(a_row["trashed_bytes"], 5000);
    assert!(
        !r.body.to_string().contains("big.bin"),
        "占比列表不得含文件名"
    );
    assert_eq!(srv.call(None, "GET", "/usage", None).status, 401);
    assert_eq!(srv.call(None, "GET", "/events/stream", None).status, 401);
}

#[test]
fn every_protected_endpoint_rejects_anonymous_and_garbage_tokens() {
    let (srv, alice, _bob) = two();
    let file = srv.upload(&alice, None, "f.txt", b"x");
    let bad = Acct {
        id: String::new(),
        token: "not-a-jwt".into(),
        email: String::new(),
    };
    let id = &file;
    let probes: Vec<(&str, String, Option<Value>)> = vec![
        ("GET", "/auth/me".into(), None),
        ("POST", "/auth/logout".into(), None),
        ("GET", "/files".into(), None),
        ("POST", "/files/upload".into(), None),
        ("POST", "/files/download".into(), Some(json!({"ids": [id]}))),
        ("GET", format!("/files/{id}"), None),
        ("GET", format!("/files/{id}/download"), None),
        ("GET", format!("/files/{id}/raw"), None),
        ("GET", format!("/files/{id}/thumbnail"), None),
        ("PATCH", format!("/files/{id}"), Some(json!({"name": "n"}))),
        (
            "POST",
            format!("/files/{id}/move"),
            Some(json!({"target_parent_id": null})),
        ),
        ("DELETE", format!("/files/{id}"), None),
        ("POST", "/folders".into(), Some(json!({"name": "x"}))),
        ("GET", "/search?q=f".into(), None),
        ("GET", "/trash".into(), None),
        ("POST", format!("/trash/{id}/restore"), None),
        ("DELETE", format!("/trash/{id}"), None),
        ("DELETE", "/trash".into(), None),
        ("GET", "/shares".into(), None),
        ("POST", "/shares".into(), Some(json!({"file_id": id}))),
        ("DELETE", format!("/shares/{id}"), None),
        ("GET", "/shares/received".into(), None),
        ("GET", "/users/directory".into(), None),
        ("GET", "/usage".into(), None),
        ("GET", "/usage/breakdown".into(), None),
        ("GET", "/events/stream".into(), None),
        ("GET", "/drives".into(), None),
        ("PATCH", "/me".into(), Some(json!({"username": "abcdef"}))),
        (
            "POST",
            "/me/password".into(),
            Some(json!({"current_password": "a", "new_password": "bbbbbbbbb"})),
        ),
    ];
    for (method, path, body) in &probes {
        let anon = srv.call(None, method, path, body.as_ref());
        assert_eq!(anon.status, 401, "匿名 {method} {path}");
        let garbage = srv.call(Some(&bad), method, path, body.as_ref());
        assert_eq!(garbage.status, 401, "垃圾令牌 {method} {path}");
    }
    // 用另一把密钥签的、看上去合法的令牌也不行。
    let now = chrono::Utc::now().timestamp();
    let forged = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &json!({"sub": alice.id, "iat": now, "exp": now + 3600}),
        &jsonwebtoken::EncodingKey::from_secret(b"some-other-secret"),
    )
    .unwrap();
    let forged = Acct {
        id: alice.id.clone(),
        token: forged,
        email: String::new(),
    };
    assert_eq!(srv.get(&forged, "/files").status, 401);
}

#[test]
fn deactivated_account_is_rejected_on_the_next_request_and_restores_cleanly() {
    let (srv, alice, bob) = two();
    let file = srv.upload(&alice, None, "keep.txt", b"keep-me");
    let trashed = srv.upload(&alice, None, "gone.txt", b"gone");
    srv.delete_file(&alice, &trashed);
    let t = srv.trash_id(&alice, &trashed);
    assert_eq!(srv.get(&alice, "/auth/me").status, 200);

    let r = srv.admin("/admin/users/delete", &json!({"identifier": alice.email}));
    assert_eq!(r.status, 200, "{}", r.body);
    // 同一个令牌，下一个请求就被拒（含文件、回收站还原、SSE）。
    for (method, path) in [
        ("GET", "/auth/me".to_string()),
        ("GET", "/files".to_string()),
        ("GET", format!("/files/{file}/download")),
        ("POST", format!("/trash/{t}/restore")),
        ("GET", "/events/stream".to_string()),
    ] {
        assert_eq!(
            srv.call(Some(&alice), method, &path, None).status,
            401,
            "{method} {path}"
        );
    }
    let login = srv.call(
        None,
        "POST",
        "/auth/login",
        Some(&json!({"identifier": alice.email, "password": PASSWORD})),
    );
    assert_eq!(login.status, 401, "已停用账户不能登录");
    // B 仍然看不到、也读不到 A 的任何东西；A 的邮箱可见性不变。
    assert_eq!(srv.get(&bob, &format!("/files/{file}")).status, 404);
    assert!(
        !srv.get(&bob, "/users/directory")
            .body
            .to_string()
            .contains("alice")
    );
    // 已停用账户的公开分享不再可用的情况见 admin 测试；这里还原账户。
    let r = srv.admin("/admin/users/restore", &json!({"identifier": alice.email}));
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(
        srv.get(&alice, "/auth/me").status,
        200,
        "还原后旧令牌恢复可用"
    );
    assert_eq!(
        srv.get(&alice, &format!("/files/{file}/download")).raw,
        b"keep-me"
    );
    assert_eq!(
        srv.get(&alice, "/trash").items().len(),
        1,
        "还原账户后回收站仍在"
    );
    // 重复还原：409。
    assert_eq!(
        srv.admin("/admin/users/restore", &json!({"identifier": alice.email}))
            .status,
        409
    );
}

#[test]
fn cookie_and_bearer_both_authenticate_and_cookie_takes_priority() {
    let (srv, alice, bob) = two();
    srv.upload(&alice, None, "a.txt", b"a");
    srv.upload(&bob, None, "b.txt", b"b");
    let cookie = format!("auth_token={}", alice.token);
    // 仅 Cookie。
    let r = srv.call_h(None, "GET", "/files", None, &[("Cookie", &cookie)]);
    assert_eq!(r.names(), ["a.txt"]);
    // 两者并存：Cookie 优先（spec）。
    let r = srv.call_h(Some(&bob), "GET", "/files", None, &[("Cookie", &cookie)]);
    assert_eq!(r.names(), ["a.txt"]);
    // 仅 Bearer。
    assert_eq!(srv.get(&bob, "/files").names(), ["b.txt"]);
}

// =====================================================================
// 优先级 2：回收站与数据安全
// =====================================================================

#[test]
fn trash_keeps_items_for_seven_days_and_restores_to_the_original_place() {
    let (srv, alice, _bob) = two();
    let folder = srv.folder(&alice, None, "Docs");
    let file = srv.upload(&alice, Some(&folder), "report.pdf", b"report-body");
    srv.delete_file(&alice, &file);

    assert!(srv.list(&alice, Some(&folder)).items().is_empty());
    let trash = srv.get(&alice, "/trash");
    let item = &trash.items()[0];
    assert_eq!(item["name"], "report.pdf");
    let deleted =
        chrono::DateTime::parse_from_rfc3339(item["deleted_at"].as_str().unwrap()).unwrap();
    let expires =
        chrono::DateTime::parse_from_rfc3339(item["expires_at"].as_str().unwrap()).unwrap();
    assert_eq!((expires - deleted).num_days(), 7, "保留期默认 7 天");

    let tid = srv.trash_id(&alice, &file);
    let r = srv.call(Some(&alice), "POST", &format!("/trash/{tid}/restore"), None);
    assert_eq!(r.status, 200);
    assert_eq!(
        (r.data()["name"].as_str(), r.data()["parent_id"].as_str()),
        (Some("report.pdf"), Some(folder.as_str()))
    );
    assert_eq!(
        srv.get(&alice, &format!("/files/{file}/download")).raw,
        b"report-body"
    );
    assert!(srv.get(&alice, "/trash").items().is_empty());
    // 同一个回收站记录不能再还原一次。
    assert_eq!(
        srv.call(Some(&alice), "POST", &format!("/trash/{tid}/restore"), None)
            .status,
        404
    );
}

#[test]
fn restore_with_name_collision_never_overwrites_and_gets_a_suffix() {
    let (srv, alice, _bob) = two();
    let old = srv.upload(&alice, None, "plan.txt", b"old");
    srv.delete_file(&alice, &old);
    let newer = srv.upload(&alice, None, "plan.txt", b"new");
    let tid = srv.trash_id(&alice, &old);

    let r = srv.call(Some(&alice), "POST", &format!("/trash/{tid}/restore"), None);
    assert_eq!(r.status, 200, "{}", r.body);
    let restored = r.data()["name"].as_str().unwrap();
    assert!(
        restored.starts_with("plan.txt (已还原 ") && restored.ends_with(')'),
        "实际：{restored}"
    );
    // 两份内容都在，互不覆盖。
    assert_eq!(
        srv.get(&alice, &format!("/files/{old}/download")).raw,
        b"old"
    );
    assert_eq!(
        srv.get(&alice, &format!("/files/{newer}/download")).raw,
        b"new"
    );
    assert_eq!(srv.list(&alice, None).items().len(), 2);
}

#[test]
fn deleting_a_folder_trashes_the_subtree_and_restore_falls_back_to_root_when_parent_is_gone() {
    let (srv, alice, _bob) = two();
    let outer = srv.folder(&alice, None, "Outer");
    let inner = srv.folder(&alice, Some(&outer), "Inner");
    let leaf = srv.upload(&alice, Some(&inner), "leaf.txt", b"leaf");
    let sib = srv.upload(&alice, Some(&outer), "sib.txt", b"sib");

    // 先删子文件夹，再删外层文件夹；还原 Inner 时它的原父目录已在回收站 → 回到根目录。
    srv.delete_file(&alice, &inner);
    assert_eq!(
        srv.get(&alice, &format!("/files/{leaf}")).status,
        404,
        "子孙随资料夹一起入回收站"
    );
    srv.delete_file(&alice, &outer);
    assert_eq!(srv.get(&alice, &format!("/files/{sib}")).status, 404);
    let tid = srv.trash_id(&alice, &inner);
    let r = srv.call(Some(&alice), "POST", &format!("/trash/{tid}/restore"), None);
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(
        r.data()["parent_id"].is_null(),
        "原目录已删，应还原到根目录"
    );
    assert_eq!(srv.list(&alice, Some(&inner)).names(), ["leaf.txt"]);
    assert_eq!(
        srv.get(&alice, &format!("/files/{leaf}/download")).raw,
        b"leaf"
    );
    // Outer 还原后里面的兄弟文件回来，但 Inner 已不在其中。
    let tid = srv.trash_id(&alice, &outer);
    srv.call(Some(&alice), "POST", &format!("/trash/{tid}/restore"), None);
    assert_eq!(srv.list(&alice, Some(&outer)).names(), ["sib.txt"]);
}

#[test]
fn permanent_delete_and_empty_trash_remove_the_bytes_from_disk() {
    let (srv, alice, _bob) = two();
    let f1 = srv.upload(&alice, None, "one.bin", &[1u8; 1000]);
    let f2 = srv.upload(&alice, None, "two.bin", &[2u8; 2000]);
    let folder = srv.folder(&alice, None, "F");
    srv.upload(&alice, Some(&folder), "three.bin", &[3u8; 300]);
    assert_eq!(srv.disk_files().len(), 3);

    srv.delete_file(&alice, &f1);
    srv.delete_file(&alice, &f2);
    srv.delete_file(&alice, &folder);
    assert_eq!(srv.disk_files().len(), 3, "进回收站不删磁盘文件");
    assert_eq!(srv.usage(&alice)["trashed_bytes"], 3300);

    let r = srv.call(
        Some(&alice),
        "DELETE",
        &format!("/trash/{}", srv.trash_id(&alice, &f1)),
        None,
    );
    assert_eq!(
        (r.status, r.data()["freed_bytes"].as_i64()),
        (200, Some(1000))
    );
    assert_eq!(srv.disk_files().len(), 2);
    assert_eq!(
        srv.call(
            Some(&alice),
            "DELETE",
            &format!("/trash/{}", srv.trash_id(&alice, &f2)),
            None
        )
        .status,
        200
    );

    let r = srv.call(Some(&alice), "DELETE", "/trash", None);
    assert_eq!(
        (
            r.data()["deleted_count"].as_i64(),
            r.data()["freed_bytes"].as_i64()
        ),
        (Some(1), Some(300))
    );
    assert!(srv.disk_files().is_empty());
    assert!(srv.get(&alice, "/trash").items().is_empty());
    assert_eq!(srv.usage(&alice)["trashed_bytes"], 0);
    assert_eq!(
        srv.call(Some(&alice), "DELETE", "/trash", None).data()["deleted_count"],
        0
    );
}

#[test]
fn expired_trash_is_purged_by_the_cleaner_and_unexpired_is_kept() {
    let (srv, alice, bob) = two();
    let old = srv.upload(&alice, None, "old.txt", b"old");
    let fresh = srv.upload(&alice, None, "fresh.txt", b"fresh");
    let bobs = srv.upload(&bob, None, "bobs.txt", b"bobs");
    for id in [&old, &fresh] {
        srv.delete_file(&alice, id);
    }
    srv.delete_file(&bob, &bobs);
    let tid = srv.trash_id(&alice, &old);
    srv.expire_trash(&tid);

    // 清理任务在进程启动时先跑一轮：重启后过期项被物理删除，未过期的原样保留。
    let dir = srv.stop();
    let srv = Server::start_in(dir, &[]);
    let alice = relogin(&srv, "alice");
    let deadline = Instant::now() + Duration::from_secs(10);
    while srv.get(&alice, "/trash").items().len() != 1 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(srv.get(&alice, "/trash").names(), ["fresh.txt"]);
    assert_eq!(
        srv.disk_files().len(),
        2,
        "过期文件的字节应被删除，另两份保留"
    );
    assert_eq!(
        srv.get(&relogin(&srv, "bob"), "/trash").names(),
        ["bobs.txt"]
    );
}

fn relogin(srv: &Server, name: &str) -> Acct {
    let email = format!("{name}@home.test");
    let r = srv.call(
        None,
        "POST",
        "/auth/login",
        Some(&json!({"identifier": email, "password": PASSWORD})),
    );
    assert_eq!(r.status, 200, "login {name}: {}", r.body);
    Acct {
        id: r.data()["user"]["id"].as_str().unwrap().to_string(),
        token: r.data()["token"].as_str().unwrap().to_string(),
        email,
    }
}

#[test]
fn admin_purge_deletes_the_trash_of_one_account_only() {
    let (srv, alice, bob) = two();
    for (who, name) in [(&alice, "a.txt"), (&bob, "b.txt")] {
        let id = srv.upload(who, None, name, b"data");
        srv.delete_file(who, &id);
    }
    let r = srv.admin("/admin/trash/purge", &json!({"identifier": alice.email}));
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(srv.get(&alice, "/trash").items().is_empty());
    assert_eq!(srv.get(&bob, "/trash").names(), ["b.txt"]);
    assert_eq!(srv.disk_files().len(), 1);
}

#[test]
fn rejected_uploads_leave_no_entry_and_no_file() {
    let (srv, alice, _bob) = two();
    // 缺 file 字段、非法名称：400，什么都不留。
    let body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nx.txt\r\n--{BOUNDARY}--\r\n"
    );
    let r = srv.send(
        "POST",
        "/files/upload",
        &[
            ("Authorization".into(), format!("Bearer {}", alice.token)),
            (
                "Content-Type".into(),
                format!("multipart/form-data; boundary={BOUNDARY}"),
            ),
        ],
        body.as_bytes(),
    );
    assert_eq!(r.status, 400);
    assert_eq!(srv.upload_raw(&alice, None, "a/b.txt", b"x").status, 400);
    assert_eq!(
        srv.upload_raw(
            &alice,
            Some(&uuid::Uuid::new_v4().to_string()),
            "x.txt",
            b"x"
        )
        .status,
        404
    );
    assert!(srv.list(&alice, None).items().is_empty());
    assert_eq!(srv.usage(&alice)["used_bytes"], 0);
    assert!(srv.disk_files().is_empty(), "被拒的上传在磁盘上留下了文件");
}

/// 已知缺陷 B-002（plan/BUGS.md）：客户端传到一半断开，已写入的字节留在磁盘上，没有任何条目指向它们。
#[test]
fn upload_interrupted_midway_leaves_no_orphan_file_on_disk() {
    let (srv, alice, _bob) = two();
    let mut s = srv.connect();
    let head = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"half.bin\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    );
    let declared = head.len() + 1_000_000 + 100;
    s.write_all(
        format!(
            "POST /api/v1/files/upload HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nContent-Type: multipart/form-data; boundary={BOUNDARY}\r\nContent-Length: {declared}\r\n\r\n{head}",
            alice.token
        )
        .as_bytes(),
    )
    .unwrap();
    s.write_all(&vec![9u8; 100_000]).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    drop(s);
    std::thread::sleep(Duration::from_secs(1));

    assert!(
        srv.list(&alice, None).items().is_empty(),
        "失败的上传不应产生条目"
    );
    assert_eq!(srv.usage(&alice)["used_bytes"], 0);
    assert!(
        srv.disk_files().is_empty(),
        "失败的上传在磁盘上留下了孤儿文件：{:?}",
        srv.disk_files()
    );
}

#[test]
fn upload_over_2mb_succeeds_and_over_limit_gets_json_413() {
    let srv = Server::start_with(&[("MAX_UPLOAD_SIZE_MB", "3")]);
    let alice = srv.signup("alice");

    // 超过 axum 默认 2 MB 限制、但在配置上限内：必须成功，且内容逐字节一致。
    let big: Vec<u8> = (0..2_500_000u32).map(|i| (i % 251) as u8).collect();
    let id = srv.upload(&alice, None, "big.bin", &big);
    assert_eq!(srv.get(&alice, &format!("/files/{id}/download")).raw, big);

    // 超过配置上限（Content-Length 已知）：服务端看到头部就拒绝，不读请求体——
    // 413 + 统一 JSON 错误信封，且不留条目。
    let mut s = srv.connect();
    s.write_all(
        format!(
            "POST /api/v1/files/upload HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nAuthorization: Bearer {}\r\nContent-Type: multipart/form-data; boundary={BOUNDARY}\r\nContent-Length: {}\r\n\r\n",
            alice.token,
            4 * 1024 * 1024
        )
        .as_bytes(),
    )
    .unwrap();
    let mut raw = Vec::new();
    let _ = s.read_to_end(&mut raw);
    let r = parse_response(&raw);
    assert_eq!(r.status, 413, "{}", String::from_utf8_lossy(&raw));
    assert_eq!(r.code(), "PAYLOAD_TOO_LARGE");
    assert_eq!(srv.list(&alice, None).items().len(), 1);

    // 分块传输（无 Content-Length）在流中途超限：同样是 JSON 413，不留孤儿文件。
    let mut s = srv.connect();
    let head = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"chunked.bin\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    );
    let _ = s.write_all(
        format!(
            "POST /api/v1/files/upload HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nAuthorization: Bearer {}\r\nContent-Type: multipart/form-data; boundary={BOUNDARY}\r\nTransfer-Encoding: chunked\r\n\r\n",
            alice.token
        )
        .as_bytes(),
    );
    let mut first = head.into_bytes();
    first.extend(vec![7u8; 1 << 20]);
    let chunks = [
        first,
        vec![7u8; 1 << 20],
        vec![7u8; 1 << 20],
        vec![7u8; 4096],
    ];
    for c in &chunks {
        if s.write_all(format!("{:x}\r\n", c.len()).as_bytes())
            .is_err()
            || s.write_all(c).is_err()
            || s.write_all(b"\r\n").is_err()
        {
            break; // 服务端已提前回应并关闭
        }
    }
    let _ = s.write_all(b"0\r\n\r\n");
    let mut raw = Vec::new();
    let _ = s.read_to_end(&mut raw);
    let r = parse_response(&raw);
    assert_eq!(r.status, 413, "{}", String::from_utf8_lossy(&raw));
    assert_eq!(r.code(), "PAYLOAD_TOO_LARGE");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(srv.list(&alice, None).items().len(), 1);
    assert_eq!(srv.disk_files().len(), 1, "超限上传在磁盘上留下了孤儿文件");
}

// =====================================================================
// 优先级 3：认证与一次性验证码
// =====================================================================

#[test]
fn register_and_login_by_email_or_phone_with_cookie_and_bearer() {
    let srv = Server::start();
    let r = srv.call(
        None,
        "POST",
        "/auth/register",
        Some(&json!({"username": "Dad", "password": PASSWORD, "email": " Dad@Home.TEST ", "phone": "+886 912-345-678"})),
    );
    assert_eq!(r.status, 201, "{}", r.body);
    let cookie = r.header("set-cookie").unwrap();
    assert!(
        cookie.starts_with("auth_token=")
            && cookie.contains("HttpOnly")
            && cookie.contains("SameSite=Lax")
            && cookie.contains("Max-Age=604800"),
        "{cookie}"
    );
    assert_eq!(r.data()["user"]["email"], "dad@home.test");
    assert_eq!(r.data()["user"]["phone"], "+886912345678");
    assert!(
        r.data()["user"].get("password").is_none() && !r.body.to_string().contains("argon2"),
        "响应不得带密码哈希"
    );

    for ident in [
        "dad@home.test",
        "DAD@home.test",
        "+886912345678",
        "+886 912 345 678",
    ] {
        let l = srv.call(
            None,
            "POST",
            "/auth/login",
            Some(&json!({"identifier": ident, "password": PASSWORD})),
        );
        assert_eq!(l.status, 200, "{ident}: {}", l.body);
        let token = l.data()["token"].as_str().unwrap();
        let me = srv.call_h(
            None,
            "GET",
            "/auth/me",
            None,
            &[("Authorization", &format!("Bearer {token}"))],
        );
        assert_eq!(me.data()["username"], "Dad");
        let me = srv.call_h(
            None,
            "GET",
            "/auth/me",
            None,
            &[("Cookie", &format!("auth_token={token}"))],
        );
        assert_eq!(me.status, 200);
    }
    // 错误凭证与不存在的账户：同样的 401 与同样的响应体（不泄露账户是否存在）。
    let wrong = srv.call(
        None,
        "POST",
        "/auth/login",
        Some(&json!({"identifier": "dad@home.test", "password": "wrong-password"})),
    );
    let ghost = srv.call(
        None,
        "POST",
        "/auth/login",
        Some(&json!({"identifier": "nobody@home.test", "password": "wrong-password"})),
    );
    assert_eq!((wrong.status, ghost.status), (401, 401));
    assert_eq!(wrong.body, ghost.body);

    // 注册校验与重复。
    for (body, status) in [
        (
            json!({"username": "x", "password": PASSWORD, "email": "a@b.test"}),
            400,
        ),
        (
            json!({"username": "Mum", "password": "short", "email": "a@b.test"}),
            400,
        ),
        (json!({"username": "Mum", "password": PASSWORD}), 400),
        (
            json!({"username": "Mum", "password": PASSWORD, "email": "not-an-email"}),
            400,
        ),
        (
            json!({"username": "Mum", "password": PASSWORD, "phone": "0912345678"}),
            400,
        ),
        (
            json!({"username": "Mum", "password": PASSWORD, "email": "dad@home.test"}),
            409,
        ),
        (
            json!({"username": "Mum", "password": PASSWORD, "phone": "+886912345678"}),
            409,
        ),
    ] {
        assert_eq!(
            srv.call(None, "POST", "/auth/register", Some(&body)).status,
            status,
            "{body}"
        );
    }

    // 登出清除 Cookie。
    let me = srv.call(
        None,
        "POST",
        "/auth/login",
        Some(&json!({"identifier": "dad@home.test", "password": PASSWORD})),
    );
    let acct = Acct {
        id: String::new(),
        token: me.data()["token"].as_str().unwrap().into(),
        email: String::new(),
    };
    let out = srv.call(Some(&acct), "POST", "/auth/logout", None);
    assert_eq!(out.status, 200);
    assert!(out.header("set-cookie").unwrap().contains("Max-Age=0"));
}

#[test]
fn closed_registration_and_admin_endpoints_are_gated() {
    let srv = Server::start_with(&[("REGISTRATION_MODE", "closed")]);
    let r = srv.call(
        None,
        "POST",
        "/auth/register",
        Some(&json!({"username": "Dad", "password": PASSWORD, "email": "d@h.test"})),
    );
    assert_eq!((r.status, r.code()), (403, "REGISTRATION_CLOSED"));
    drop(srv);

    let off = Server::start_with(&[("ADMIN_PASSWORD", "")]);
    assert_eq!(
        off.admin("/admin/users/delete", &json!({"identifier": "x@y.test"}))
            .status,
        404,
        "未配置口令 = 路由不存在"
    );
    drop(off);

    let (srv, alice, _bob) = two();
    let body = json!({"identifier": alice.email});
    assert_eq!(
        srv.call(None, "POST", "/admin/users/delete", Some(&body))
            .status,
        401,
        "缺口令"
    );
    assert_eq!(
        srv.call_h(
            None,
            "POST",
            "/admin/users/delete",
            Some(&body),
            &[("X-Admin-Password", "wrong")]
        )
        .status,
        401
    );
    let remote = [("X-Admin-Password", ADMIN_PW), ("X-Forwarded-For", REMOTE)];
    assert_eq!(
        srv.call_h(None, "POST", "/admin/users/delete", Some(&body), &remote)
            .status,
        403,
        "公网来源"
    );
    assert_eq!(
        srv.call_h(None, "GET", "/admin/users", None, &remote)
            .status,
        403
    );
    assert_eq!(
        srv.call_h(None, "POST", "/admin/trash/purge", Some(&body), &remote)
            .status,
        403
    );
    // 普通账户令牌不是管理员凭证。
    assert_eq!(
        srv.call(Some(&alice), "GET", "/admin/users", None).status,
        401
    );
    assert_eq!(
        srv.get(&alice, "/auth/me").status,
        200,
        "被拒的管理调用不能影响账户"
    );
    // 本机 + 正确口令：列表含两个账户。
    let list = srv.call_h(
        None,
        "GET",
        "/admin/users",
        None,
        &[("X-Admin-Password", ADMIN_PW)],
    );
    assert_eq!((list.status, list.items().len()), (200, 2));
    assert_eq!(
        srv.admin(
            "/admin/users/delete",
            &json!({"identifier": "ghost@home.test"})
        )
        .status,
        404
    );
}

#[test]
fn otp_allows_five_attempts_and_three_codes_per_hour_and_purposes_are_isolated() {
    let (srv, alice, _bob) = two();
    let challenge = |who: &Acct| {
        srv.call(
            Some(who),
            "POST",
            "/me/identifier/challenge",
            Some(&json!({"provider": "email"})),
        )
    };

    // 5 次错码之后，即便第 6 次给对码也被拒（码作废，需重新获取）。
    assert_eq!(challenge(&alice).status, 204);
    let code = srv.otp_code(&alice.email);
    let change = |who: &Acct, code: &str, new: &str| {
        srv.call(
            Some(who),
            "PUT",
            "/me/identifier",
            Some(&json!({"provider": "email", "new_value": new, "current_password": PASSWORD, "old_code": code})),
        )
    };
    let wrong = if code == "000000" { "111111" } else { "000000" };
    for i in 0..5 {
        let r = change(&alice, wrong, "new@home.test");
        assert_eq!(r.status, 400, "第 {} 次错码", i + 1);
    }
    let r = change(&alice, &code, "new@home.test");
    assert_eq!(r.status, 400, "第 6 次即使码正确也必须被拒：{}", r.body);
    assert_eq!(
        srv.get(&alice, "/auth/me").data()["email"],
        alice.email,
        "标识不得被改"
    );

    // 同一目标每小时最多 3 条：已发 1 条，再发 2 条成功，第 4 条被拒。
    assert_eq!(challenge(&alice).status, 204);
    assert_eq!(challenge(&alice).status, 204);
    assert_eq!(
        challenge(&alice).status,
        400,
        "同一目标 1 小时内第 4 条验证码必须被拒"
    );
    // 限流按目标计，不按账户：另一账户对它自己的邮箱不受影响。
    let carol = srv.signup("carol");
    assert_eq!(challenge(&carol).status, 204);

    // 用途隔离：identifier_old 的码不能拿去做 identifier_verify。
    let code = srv.otp_code(&carol.email);
    let r = srv.call(
        Some(&carol),
        "POST",
        "/me/identifier/verify",
        Some(&json!({"provider": "email", "code": code})),
    );
    assert_eq!(r.status, 400, "challenge 的码不能用于验证标识：{}", r.body);
}

#[test]
fn identifier_change_needs_password_and_a_code_from_the_old_channel() {
    let (srv, alice, _bob) = two();
    let put = |body: Value| srv.call(Some(&alice), "PUT", "/me/identifier", Some(&body));

    // 没有验证码：被拒（只有密码不足以改标识）。
    let r = put(
        json!({"provider": "email", "new_value": "evil@home.test", "current_password": PASSWORD, "old_code": "123456"}),
    );
    assert_eq!(r.status, 400);
    srv.call(
        Some(&alice),
        "POST",
        "/me/identifier/challenge",
        Some(&json!({"provider": "email"})),
    );
    let code = srv.otp_code(&alice.email);
    // 有码但密码错：被拒，且码不应因此被消耗。
    let r = put(
        json!({"provider": "email", "new_value": "new@home.test", "current_password": "wrong-password", "old_code": code}),
    );
    assert!(
        r.status == 401 || r.status == 403 || r.status == 400,
        "密码错必须被拒：{} {}",
        r.status,
        r.body
    );
    assert_eq!(srv.get(&alice, "/auth/me").data()["email"], alice.email);
    // 新值格式不对：400。
    let r = put(
        json!({"provider": "email", "new_value": "nope", "current_password": PASSWORD, "old_code": code}),
    );
    assert_eq!(r.status, 400);
    // 删除唯一的标识：被拒（至少保留一个）；这次校验已消耗该码，之后需重新获取。
    let r = srv.call(
        Some(&alice),
        "DELETE",
        "/me/identifier",
        Some(&json!({"provider": "email", "current_password": PASSWORD, "old_code": code})),
    );
    assert!(r.status >= 400, "不能删除最后一个标识：{}", r.body);
    assert_eq!(srv.get(&alice, "/auth/me").data()["email"], alice.email);
    srv.call(
        Some(&alice),
        "POST",
        "/me/identifier/challenge",
        Some(&json!({"provider": "email"})),
    );
    let code = srv.otp_code(&alice.email);

    // 两个因素齐全：成功；新标识未验证；旧标识不能再登录；新标识可登录。
    let r = put(
        json!({"provider": "email", "new_value": "New@Home.test", "current_password": PASSWORD, "old_code": code}),
    );
    assert_eq!(r.status, 200, "{}", r.body);
    let ident = r.data()["identities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["provider"] == "email")
        .unwrap()
        .clone();
    assert_eq!(
        (ident["identifier"].as_str(), ident["verified"].as_bool()),
        (Some("new@home.test"), Some(false))
    );
    let old = srv.call(
        None,
        "POST",
        "/auth/login",
        Some(&json!({"identifier": alice.email, "password": PASSWORD})),
    );
    assert_eq!(old.status, 401);
    let new = srv.call(
        None,
        "POST",
        "/auth/login",
        Some(&json!({"identifier": "new@home.test", "password": PASSWORD})),
    );
    assert_eq!(new.status, 200);
    // 码是一次性的：同一个码不能再用第二次。
    let r = put(
        json!({"provider": "email", "new_value": "again@home.test", "current_password": PASSWORD, "old_code": code}),
    );
    assert_eq!(r.status, 400);

    // 验证新标识：request → verify 置 verified。
    assert_eq!(
        srv.call(
            Some(&alice),
            "POST",
            "/me/identifier/verify/request",
            Some(&json!({"provider": "email"}))
        )
        .status,
        204
    );
    let vcode = srv.otp_code("new@home.test");
    let r = srv.call(
        Some(&alice),
        "POST",
        "/me/identifier/verify",
        Some(&json!({"provider": "email", "code": vcode})),
    );
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(
        r.data()["identities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["provider"] == "email" && i["verified"] == true)
    );
    assert_eq!(
        srv.call(
            Some(&alice),
            "POST",
            "/me/identifier/verify/request",
            Some(&json!({"provider": "email"}))
        )
        .status,
        409,
        "已验证的不能再请求"
    );

    // 新标识若已被他人占用：冲突。
    srv.call(
        Some(&alice),
        "POST",
        "/me/identifier/challenge",
        Some(&json!({"provider": "email"})),
    );
    let code = srv.otp_code("new@home.test");
    let r = put(
        json!({"provider": "email", "new_value": "bob@home.test", "current_password": PASSWORD, "old_code": code}),
    );
    assert_eq!(r.status, 409, "{}", r.body);
}

#[test]
fn profile_name_and_password_changes_apply_immediately() {
    let (srv, alice, _bob) = two();
    let r = srv.call(
        Some(&alice),
        "PATCH",
        "/me",
        Some(&json!({"username": "Alice Zhang"})),
    );
    assert_eq!(
        (r.status, r.data()["username"].as_str()),
        (200, Some("Alice Zhang"))
    );
    assert_eq!(
        srv.call(
            Some(&alice),
            "PATCH",
            "/me",
            Some(&json!({"username": "ab"}))
        )
        .status,
        400
    );

    let bad = srv.call(
        Some(&alice),
        "POST",
        "/me/password",
        Some(&json!({"current_password": "wrong-password", "new_password": "brand-new-password"})),
    );
    assert!(bad.status == 401 || bad.status == 403 || bad.status == 400);
    let short = srv.call(
        Some(&alice),
        "POST",
        "/me/password",
        Some(&json!({"current_password": PASSWORD, "new_password": "short"})),
    );
    assert_eq!(short.status, 400);
    let ok = srv.call(
        Some(&alice),
        "POST",
        "/me/password",
        Some(&json!({"current_password": PASSWORD, "new_password": "brand-new-password"})),
    );
    assert_eq!(ok.status, 200);
    let old = srv.call(
        None,
        "POST",
        "/auth/login",
        Some(&json!({"identifier": alice.email, "password": PASSWORD})),
    );
    assert_eq!(old.status, 401);
    let new = srv.call(
        None,
        "POST",
        "/auth/login",
        Some(&json!({"identifier": alice.email, "password": "brand-new-password"})),
    );
    assert_eq!(new.status, 200);
}

// =====================================================================
// 优先级 4：传输
// =====================================================================

#[test]
fn upload_dedupes_names_and_rename_move_conflicts_are_rejected() {
    let (srv, alice, bob) = two();
    srv.upload(&alice, None, "photo.jpg", b"1");
    let second = srv.upload_raw(&alice, None, "photo.jpg", b"2");
    assert_eq!(second.data()["name"], "photo(1).jpg");
    assert_eq!(
        srv.upload_raw(&alice, None, "photo.jpg", b"3").data()["name"],
        "photo(2).jpg"
    );
    srv.folder(&alice, None, "Docs");
    let d = srv.call(
        Some(&alice),
        "POST",
        "/folders",
        Some(&json!({"name": "Docs"})),
    );
    assert_eq!(d.data()["name"], "Docs(1)");
    // 别的账户的同名不算冲突。
    assert_eq!(
        srv.upload_raw(&bob, None, "photo.jpg", b"b").data()["name"],
        "photo.jpg"
    );

    let id = second.data()["id"].as_str().unwrap();
    let r = srv.call(
        Some(&alice),
        "PATCH",
        &format!("/files/{id}"),
        Some(&json!({"name": "photo.jpg"})),
    );
    assert_eq!(r.status, 409);
    let r = srv.call(
        Some(&alice),
        "PATCH",
        &format!("/files/{id}"),
        Some(&json!({"name": "renamed.jpg"})),
    );
    assert_eq!(
        (r.status, r.data()["name"].as_str()),
        (200, Some("renamed.jpg"))
    );
    for bad in ["", "  ", "a/b", "a\\b"] {
        assert_eq!(
            srv.call(
                Some(&alice),
                "PATCH",
                &format!("/files/{id}"),
                Some(&json!({"name": bad}))
            )
            .status,
            400,
            "{bad:?}"
        );
    }

    // 移动：同名冲突 409；移到自己的子孙 400；成功后位置变化。
    let outer = srv.folder(&alice, None, "Outer");
    let inner = srv.folder(&alice, Some(&outer), "Inner");
    srv.upload(&alice, Some(&outer), "renamed.jpg", b"clash");
    let mv = |id: &str, target: Option<&str>| {
        srv.call(
            Some(&alice),
            "POST",
            &format!("/files/{id}/move"),
            Some(&json!({"target_parent_id": target})),
        )
    };
    assert_eq!(mv(id, Some(&outer)).status, 409);
    assert_eq!(
        mv(&outer, Some(&inner)).status,
        400,
        "资料夹不能移进自己的子孙"
    );
    assert_eq!(mv(&outer, Some(&outer)).status, 400);
    assert_eq!(mv(id, Some(&inner)).status, 200);
    assert_eq!(srv.list(&alice, Some(&inner)).names(), ["renamed.jpg"]);
    assert_eq!(mv(id, Some(&uuid::Uuid::new_v4().to_string())).status, 404);
    assert_eq!(mv(id, None).status, 200);
}

#[test]
fn range_requests_return_206_with_exact_bytes_on_download_and_raw() {
    let (srv, alice, _bob) = two();
    let content: Vec<u8> = (0u8..=99).collect();
    let id = srv.upload(&alice, None, "bytes.bin", &content);

    for tail in ["download", "raw"] {
        let path = format!("/files/{id}/{tail}");
        let range = |h: &str| srv.call_h(Some(&alice), "GET", &path, None, &[("Range", h)]);

        let full = srv.get(&alice, &path);
        assert_eq!(
            (full.status, full.raw.as_slice()),
            (200, content.as_slice())
        );
        assert_eq!(full.header("accept-ranges"), Some("bytes"));
        assert_eq!(full.header("content-length"), Some("100"));
        let disposition = full.header("content-disposition").unwrap_or("");
        assert!(
            disposition.starts_with(if tail == "raw" {
                "inline"
            } else {
                "attachment"
            }),
            "{tail}: {disposition}"
        );

        let r = range("bytes=10-19");
        assert_eq!(
            (r.status, r.raw.as_slice()),
            (206, &content[10..20]),
            "{tail}"
        );
        assert_eq!(r.header("content-range"), Some("bytes 10-19/100"));
        assert_eq!(r.header("content-length"), Some("10"));
        let r = range("bytes=90-");
        assert_eq!((r.status, r.raw.as_slice()), (206, &content[90..]));
        assert_eq!(r.header("content-range"), Some("bytes 90-99/100"));
        let r = range("bytes=-5");
        assert_eq!(
            (r.status, r.raw.as_slice()),
            (206, &content[95..]),
            "后缀 Range"
        );
        assert_eq!(r.header("content-range"), Some("bytes 95-99/100"));
        let r = range("bytes=0-0");
        assert_eq!((r.status, r.raw.as_slice()), (206, &content[..1]));
        // 终点越界被截断到文件末尾。
        let r = range("bytes=95-5000");
        assert_eq!((r.status, r.raw.as_slice()), (206, &content[95..]));
        // 起点越界：416。
        let r = range("bytes=100-");
        assert_eq!(r.status, 416);
        assert_eq!(range("bytes=500-600").status, 416);
    }
    // 文件夹不能下载 / 预览。
    let folder = srv.folder(&alice, None, "D");
    assert_eq!(
        srv.get(&alice, &format!("/files/{folder}/download")).status,
        400
    );
    assert_eq!(srv.get(&alice, &format!("/files/{folder}/raw")).status, 400);
}

#[test]
fn batch_download_streams_zip_and_tar_gz_with_nested_folders() {
    let (srv, alice, bob) = two();
    let folder = srv.folder(&alice, None, "Trip");
    let sub = srv.folder(&alice, Some(&folder), "Day1");
    srv.upload(&alice, Some(&sub), "a.txt", b"alpha-content-alpha-content");
    srv.upload(&alice, Some(&folder), "b.txt", b"bravo");
    let top = srv.upload(&alice, None, "top.txt", b"top-level");
    let gone = srv.upload(&alice, None, "deleted.txt", b"in-trash");
    srv.delete_file(&alice, &gone);
    srv.upload(&bob, None, "bob.txt", b"bobs");

    let ids = json!([folder, top]);
    let zip = srv.call(
        Some(&alice),
        "POST",
        "/files/download",
        Some(&json!({"ids": ids})),
    );
    assert_eq!(zip.status, 200);
    assert_eq!(zip.header("content-type"), Some("application/zip"));
    assert!(
        zip.header("content-disposition")
            .unwrap()
            .contains("cloud_home_download.zip")
    );
    let mut entries = read_zip(&zip.raw);
    entries.sort();
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["Trip/Day1/a.txt", "Trip/b.txt", "top.txt"]);
    assert_eq!(entries[0].1, b"alpha-content-alpha-content");
    assert_eq!(entries[2].1, b"top-level");

    let tgz = srv.call(
        Some(&alice),
        "POST",
        "/files/download",
        Some(&json!({"ids": ids, "format": "tar.gz"})),
    );
    assert_eq!(tgz.status, 200);
    assert!(
        tgz.header("content-disposition")
            .unwrap()
            .contains("cloud_home_download.tar.gz")
    );
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(tgz.raw.as_slice()));
    let mut got: Vec<(String, Vec<u8>)> = archive
        .entries()
        .unwrap()
        .map(|e| {
            let mut e = e.unwrap();
            let name = e
                .path()
                .unwrap()
                .to_string_lossy()
                .trim_end_matches('/')
                .to_string();
            let mut buf = Vec::new();
            e.read_to_end(&mut buf).unwrap();
            (name, buf)
        })
        .filter(|(_, b)| !b.is_empty())
        .collect();
    got.sort();
    assert_eq!(
        got.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        ["Trip/Day1/a.txt", "Trip/b.txt", "top.txt"]
    );
    assert_eq!(got[1].1, b"bravo");

    // 校验：空 ids、只选了回收站里的、未知格式。
    assert_eq!(
        srv.call(
            Some(&alice),
            "POST",
            "/files/download",
            Some(&json!({"ids": []}))
        )
        .status,
        400
    );
    assert_eq!(
        srv.call(
            Some(&alice),
            "POST",
            "/files/download",
            Some(&json!({"ids": [gone]}))
        )
        .status,
        404
    );
    assert!(
        srv.call(
            Some(&alice),
            "POST",
            "/files/download",
            Some(&json!({"ids": [top], "format": "rar"}))
        )
        .status
            >= 400
    );
}

/// 读取 ZIP（用中央目录，兼容流式写入的 data descriptor 与 zip64 扩展），返回 (名称, 解压后内容)。
fn read_zip(zip: &[u8]) -> Vec<(String, Vec<u8>)> {
    let u16le = |o: usize| u16::from_le_bytes([zip[o], zip[o + 1]]) as usize;
    let u32le = |o: usize| u32::from_le_bytes(zip[o..o + 4].try_into().unwrap()) as usize;
    let u64le = |o: usize| u64::from_le_bytes(zip[o..o + 8].try_into().unwrap()) as usize;
    let eocd = zip
        .windows(4)
        .rposition(|w| w == b"PK\x05\x06")
        .expect("缺少 ZIP 结尾记录");
    let (mut count, mut pos) = (u16le(eocd + 10), u32le(eocd + 16));
    if let Some(z64) = zip.windows(4).rposition(|w| w == b"PK\x06\x06") {
        (count, pos) = (u64le(z64 + 32), u64le(z64 + 48));
    }
    let mut out = Vec::new();
    for _ in 0..count {
        assert_eq!(&zip[pos..pos + 4], b"PK\x01\x02");
        let method = u16le(pos + 10);
        let (mut comp, nlen, elen, clen, mut local) = (
            u32le(pos + 20),
            u16le(pos + 28),
            u16le(pos + 30),
            u16le(pos + 32),
            u32le(pos + 42),
        );
        let name = String::from_utf8(zip[pos + 46..pos + 46 + nlen].to_vec()).unwrap();
        // zip64 扩展（id 0x0001）：按顺序给出被标成 0xFFFFFFFF 的 原始大小 / 压缩大小 / 偏移。
        let mut extra = pos + 46 + nlen;
        let extra_end = extra + elen;
        let (uncomp_max, comp_max) = (u32le(pos + 24) == 0xFFFF_FFFF, comp == 0xFFFF_FFFF);
        while extra + 4 <= extra_end {
            let (id, len) = (u16le(extra), u16le(extra + 2));
            if id == 1 {
                let mut at = extra + 4;
                if uncomp_max {
                    at += 8;
                }
                if comp_max {
                    comp = u64le(at);
                    at += 8;
                }
                if local == 0xFFFF_FFFF {
                    local = u64le(at);
                }
            }
            extra += 4 + len;
        }
        let data_at = local + 30 + u16le(local + 26) + u16le(local + 28);
        let stored = &zip[data_at..data_at + comp];
        let data = match method {
            0 => stored.to_vec(),
            8 => {
                let mut v = Vec::new();
                flate2::read::DeflateDecoder::new(stored)
                    .read_to_end(&mut v)
                    .unwrap();
                v
            }
            m => panic!("未知压缩方式 {m}"),
        };
        if !name.ends_with('/') {
            out.push((name, data));
        }
        pos += 46 + nlen + elen + clen;
    }
    out
}

#[test]
fn thumbnails_are_jpeg_in_allowed_sizes_and_reject_non_images() {
    let (srv, alice, _bob) = two();
    let img = srv.upload(&alice, None, "pic.png", &png_bytes());
    let txt = srv.upload(&alice, None, "note.txt", b"text");
    let broken = srv.upload(&alice, None, "broken.png", b"not really a png");

    for size in [64, 256, 1024] {
        let r = srv.get(&alice, &format!("/files/{img}/thumbnail?size={size}"));
        assert_eq!(r.status, 200, "size {size}");
        assert_eq!(r.header("content-type"), Some("image/jpeg"));
        assert_eq!(&r.raw[..3], [0xFF, 0xD8, 0xFF], "JPEG 魔数");
        let decoded = image::load_from_memory(&r.raw).unwrap();
        assert!(decoded.width().max(decoded.height()) <= size);
    }
    let default = srv.get(&alice, &format!("/files/{img}/thumbnail"));
    assert_eq!(default.status, 200, "缺省尺寸 256");
    assert_eq!(image::load_from_memory(&default.raw).unwrap().width(), 256);
    // 缓存命中：第二次拿到同样的字节。
    assert_eq!(
        srv.get(&alice, &format!("/files/{img}/thumbnail")).raw,
        default.raw
    );

    for bad in ["size=100", "size=0", "size=99999", "size=abc"] {
        assert_eq!(
            srv.get(&alice, &format!("/files/{img}/thumbnail?{bad}"))
                .status,
            400,
            "{bad}"
        );
    }
    assert_eq!(
        srv.get(&alice, &format!("/files/{txt}/thumbnail")).status,
        400,
        "非图片"
    );
    assert_eq!(
        srv.get(&alice, &format!("/files/{broken}/thumbnail"))
            .status,
        400,
        "解不开的图片"
    );
    // 文件进了回收站后缩略图也不可取。
    srv.delete_file(&alice, &img);
    assert_eq!(
        srv.get(&alice, &format!("/files/{img}/thumbnail")).status,
        404
    );
}

// =====================================================================
// 分享链接
// =====================================================================

#[test]
fn public_share_lifecycle_view_download_range_count_expiry_and_revoke() {
    let (srv, alice, _bob) = two();
    let file = srv.upload(&alice, None, "photo.txt", b"0123456789");
    let share = srv.share(&alice, &file, json!({}));
    let token = share["token"].as_str().unwrap();
    assert!(
        share["url"]
            .as_str()
            .unwrap()
            .ends_with(&format!("/api/v1/public/shares/{token}"))
    );
    assert!(share["recipients"].as_array().unwrap().is_empty());
    assert_eq!(share["can_download"], true);

    // 匿名可看信息（含分享者名）并下载，支持 Range，且每次下载 +1 访问数。
    let info = srv.call(None, "GET", &format!("/public/shares/{token}"), None);
    assert_eq!(
        (
            info.data()["name"].as_str(),
            info.data()["shared_by"].as_str()
        ),
        (Some("photo.txt"), Some("alice"))
    );
    let dl = srv.call(
        None,
        "GET",
        &format!("/public/shares/{token}/download"),
        None,
    );
    assert_eq!(
        (dl.status, dl.raw.as_slice()),
        (200, b"0123456789".as_slice())
    );
    let part = srv.call_h(
        None,
        "GET",
        &format!("/public/shares/{token}/download"),
        None,
        &[("Range", "bytes=2-4")],
    );
    assert_eq!((part.status, part.raw.as_slice()), (206, b"234".as_slice()));
    assert_eq!(part.header("content-range"), Some("bytes 2-4/10"));
    assert_eq!(srv.get(&alice, "/shares").items()[0]["access_count"], 2);

    // 公开链接不暴露账户内其它端点：用 token 当文件 id 没有任何意义。
    assert_eq!(
        srv.call(None, "GET", &format!("/files/{token}"), None)
            .status,
        401
    );

    // 撤销：立即 404。
    let sid = share["id"].as_str().unwrap();
    assert_eq!(
        srv.call(Some(&alice), "DELETE", &format!("/shares/{sid}"), None)
            .status,
        200
    );
    assert_eq!(
        srv.call(None, "GET", &format!("/public/shares/{token}"), None)
            .status,
        404
    );
    assert_eq!(
        srv.call(
            None,
            "GET",
            &format!("/public/shares/{token}/download"),
            None
        )
        .status,
        404
    );
    assert_eq!(
        srv.call(Some(&alice), "DELETE", &format!("/shares/{sid}"), None)
            .status,
        200,
        "重复撤销不应出错"
    );
}

#[test]
fn share_expiry_gives_410_and_can_download_false_gives_403() {
    let (srv, alice, _bob) = two();
    let file = srv.upload(&alice, None, "doc.txt", b"doc");

    let expired = srv.share(&alice, &file, json!({"expires_at": "2000-01-01T00:00:00Z"}));
    let t = expired["token"].as_str().unwrap();
    assert_eq!(
        srv.call(None, "GET", &format!("/public/shares/{t}"), None)
            .status,
        410
    );
    assert_eq!(
        srv.call(None, "GET", &format!("/public/shares/{t}/download"), None)
            .status,
        410
    );

    let future = (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339();
    let valid = srv.share(&alice, &file, json!({"expires_at": future}));
    assert_eq!(
        srv.call(
            None,
            "GET",
            &format!(
                "/public/shares/{}/download",
                valid["token"].as_str().unwrap()
            ),
            None
        )
        .status,
        200
    );

    let view_only = srv.share(&alice, &file, json!({"can_download": false}));
    let t = view_only["token"].as_str().unwrap();
    let info = srv.call(None, "GET", &format!("/public/shares/{t}"), None);
    assert_eq!(
        (info.status, info.data()["can_download"].as_bool()),
        (200, Some(false))
    );
    let dl = srv.call(None, "GET", &format!("/public/shares/{t}/download"), None);
    assert_eq!((dl.status, dl.code()), (403, "FORBIDDEN"));
    assert!(!String::from_utf8_lossy(&dl.raw).contains("doc"));
    assert_eq!(
        srv.get(&alice, "/shares")
            .items()
            .iter()
            .map(|i| i["access_count"].as_i64().unwrap())
            .sum::<i64>(),
        1,
        "被拒绝的下载不计数"
    );

    assert_eq!(
        srv.call(
            Some(&alice),
            "POST",
            "/shares",
            Some(&json!({"file_id": file, "expires_at": "tomorrow"}))
        )
        .status,
        400
    );
    assert_eq!(
        srv.call(
            Some(&alice),
            "POST",
            "/shares",
            Some(&json!({"file_id": uuid::Uuid::new_v4().to_string()}))
        )
        .status,
        404
    );
}

#[test]
fn public_share_stops_working_when_the_file_is_trashed_or_the_owner_is_deactivated() {
    let (srv, alice, _bob) = two();
    let file = srv.upload(&alice, None, "doc.txt", b"doc");
    let t = srv.share(&alice, &file, json!({}))["token"]
        .as_str()
        .unwrap()
        .to_string();
    let url = format!("/public/shares/{t}/download");
    assert_eq!(srv.call(None, "GET", &url, None).status, 200);

    srv.delete_file(&alice, &file);
    assert_eq!(
        srv.call(None, "GET", &url, None).status,
        404,
        "文件在回收站里，分享不应再可下载"
    );
    let tid = srv.trash_id(&alice, &file);
    srv.call(Some(&alice), "POST", &format!("/trash/{tid}/restore"), None);
    assert_eq!(
        srv.call(None, "GET", &url, None).status,
        200,
        "还原后分享恢复"
    );

    srv.admin("/admin/users/delete", &json!({"identifier": alice.email}));
    let r = srv.call(None, "GET", &url, None);
    assert_eq!(r.status, 404, "账户被停用后其公开分享不应继续提供文件");
}

#[test]
fn share_notifications_reach_only_the_recipient_over_sse() {
    let srv = Server::start();
    let alice = srv.signup("alice");
    let bob = srv.signup("bob");
    let carol = srv.signup("carol");
    let mut bob_stream = srv.events(&bob);
    let mut carol_stream = srv.events(&carol);
    let file = srv.upload(&alice, None, "f.txt", b"x");

    let share = srv.share(&alice, &file, json!({"recipient_ids": [bob.id]}));
    let sid = share["id"].as_str().unwrap();
    let got = bob_stream.wait_for("share.received", |d| d["share_id"] == sid);
    assert!(got.is_some(), "收件人没收到 share.received");
    assert!(
        !carol_stream.sees("share.received", Duration::from_secs(1)),
        "无关账户收到了分享通知"
    );

    srv.call(Some(&alice), "DELETE", &format!("/shares/{sid}"), None);
    assert!(
        bob_stream
            .wait_for("share.revoked", |d| d["share_id"] == sid)
            .is_some()
    );
    assert!(!carol_stream.sees("share.revoked", Duration::from_secs(1)));
}

// =====================================================================
// 补充（R-030 第二轮）：收件人越权、回收站可达性、恶意文件名、令牌伪造、Range 边界、
// 批量下载入参、管理员列表泄露、SSE 变更种类
// =====================================================================

/// 站内定向分享只给「通过分享路径读取」的权利：收件人拿到文件 / 资料夹 id 后，
/// 不能用属主端点读、改、移、删、再分享、批量下载或搜索到它们。
#[test]
fn a_share_recipient_gets_no_owner_level_access_to_the_shared_items() {
    let (srv, alice, bob) = two();
    let folder = srv.folder(&alice, None, "Shared");
    let inside = srv.upload(&alice, Some(&folder), "inside.txt", b"inside-secret");
    let image = srv.upload(&alice, None, "pic.png", &png_bytes());
    srv.share(&alice, &folder, json!({"recipient_ids": [bob.id]}));
    srv.share(&alice, &image, json!({"recipient_ids": [bob.id]}));

    // 前提：收件人确实能经分享路径读到（否则下面的 404 什么也证明不了）。
    assert_eq!(srv.get(&bob, "/shares/received").items().len(), 2);

    let ghost = uuid::Uuid::new_v4().to_string();
    for id in [&folder, &inside, &image] {
        for (method, tail, body) in [
            ("GET", "", None),
            ("GET", "/download", None),
            ("GET", "/raw", None),
            ("GET", "/thumbnail", None),
            ("PATCH", "", Some(json!({"name": "hijacked"}))),
            ("POST", "/move", Some(json!({"target_parent_id": null}))),
            ("DELETE", "", None),
        ] {
            let r = srv.call(
                Some(&bob),
                method,
                &format!("/files/{id}{tail}"),
                body.as_ref(),
            );
            let g = srv.call(
                Some(&bob),
                method,
                &format!("/files/{ghost}{tail}"),
                body.as_ref(),
            );
            assert_eq!(
                r.status, 404,
                "收件人 {method} /files/{id}{tail}: {}",
                r.body
            );
            assert_eq!(
                r.body, g.body,
                "收件人 {method} {tail}：与不存在的 id 响应不同"
            );
        }
        // 不能把别人的东西再分享出去（含公开链接）。
        let r = srv.call(Some(&bob), "POST", "/shares", Some(&json!({"file_id": id})));
        assert_eq!(r.status, 404, "收件人转分享 {id}: {}", r.body);
    }
    assert_eq!(srv.list(&bob, Some(&folder)).status, 404);
    let r = srv.call(
        Some(&bob),
        "POST",
        "/files/download",
        Some(&json!({"ids": [inside], "format": "zip"})),
    );
    assert_eq!(r.status, 404);
    assert!(srv.get(&bob, "/search?q=inside").items().is_empty());
    assert!(srv.get(&bob, "/search?q=pic").items().is_empty());
    assert!(srv.list(&bob, None).items().is_empty());

    // 属主的东西原封未动。
    assert_eq!(srv.list(&alice, Some(&folder)).names(), ["inside.txt"]);
    assert_eq!(
        srv.get(&alice, &format!("/files/{inside}/download")).raw,
        b"inside-secret"
    );
    assert_eq!(srv.get(&alice, "/shares").items().len(), 2);
}

/// 进了回收站的东西：本人和外人都读不到、下不了、搜不到、建不了分享；还原后一切恢复。
#[test]
fn trashed_items_are_unreachable_unshareable_and_unsearchable_until_restored() {
    let (srv, alice, bob) = two();
    let folder = srv.folder(&alice, None, "Box");
    let inside = srv.upload(&alice, Some(&folder), "needle.txt", b"in-the-box");
    let image = srv.upload(&alice, None, "needle.png", &png_bytes());
    srv.delete_file(&alice, &folder);
    srv.delete_file(&alice, &image);

    for who in [&alice, &bob] {
        for id in [&folder, &inside, &image] {
            for tail in ["", "/download", "/raw", "/thumbnail"] {
                let r = srv.get(who, &format!("/files/{id}{tail}"));
                assert_eq!(r.status, 404, "{} 读回收站里的 {id}{tail}", who.email);
            }
        }
        assert!(
            srv.get(who, "/search?q=needle").items().is_empty(),
            "{}",
            who.email
        );
    }
    assert_eq!(srv.list(&alice, Some(&folder)).status, 404);
    for id in [&folder, &inside, &image] {
        let r = srv.call(
            Some(&alice),
            "POST",
            "/shares",
            Some(&json!({"file_id": id})),
        );
        assert_eq!(r.status, 404, "给回收站里的 {id} 建分享：{}", r.body);
    }
    let r = srv.call(
        Some(&alice),
        "POST",
        "/files/download",
        Some(&json!({"ids": [image], "format": "zip"})),
    );
    assert_eq!(r.status, 404, "批量下载回收站里的东西");
    // 二次删除同一个东西：404，回收站不重复。
    assert_eq!(
        srv.call(Some(&alice), "DELETE", &format!("/files/{image}"), None)
            .status,
        404
    );
    assert_eq!(srv.get(&alice, "/trash").items().len(), 2);

    // 还原：整棵子树回来，内容完好。
    let t = srv.trash_id(&alice, &folder);
    let r = srv.call(Some(&alice), "POST", &format!("/trash/{t}/restore"), None);
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(
        srv.get(&alice, &format!("/files/{inside}/download")).raw,
        b"in-the-box"
    );
    assert_eq!(srv.get(&alice, "/search?q=needle").items().len(), 1);
    // 已还原的条目再还原：404。
    assert_eq!(
        srv.call(Some(&alice), "POST", &format!("/trash/{t}/restore"), None)
            .status,
        404
    );
}

/// 恶意或古怪的文件名：绝不能写出账户目录；点文件与无扩展名文件的重名也要得到 `(1)`。
#[test]
fn hostile_upload_names_stay_inside_the_account_and_odd_names_still_dedupe() {
    let (srv, alice, _bob) = two();
    for name in [
        "../../escape.txt",
        "..\\..\\escape2.txt",
        "/etc/escape3.txt",
        "..",
        ".",
    ] {
        let r = srv.upload_raw(&alice, None, name, b"payload");
        assert!(
            r.status == 400
                || (r.status == 201 && !r.data()["name"].as_str().unwrap().contains(['/', '\\'])),
            "{name:?} -> {} {}",
            r.status,
            r.body
        );
    }
    let user_root = srv.dir.join("drive/cloud_home_data/users").join(&alice.id);
    for f in srv.disk_files() {
        assert!(
            f.starts_with(&user_root),
            "文件写到了账户目录之外：{}",
            f.display()
        );
    }
    let mut everything = Vec::new();
    walk(&srv.dir.join("drive"), &mut everything);
    assert!(
        everything
            .iter()
            .all(|p| !p.to_string_lossy().contains("escape")),
        "目录穿越：{everything:?}"
    );

    for name in ["README", ".env", "archive.tar.gz", "名字 with spaces.txt"] {
        assert_eq!(
            srv.upload_raw(&alice, None, name, b"1").data()["name"],
            name
        );
        let again = srv.upload_raw(&alice, None, name, b"2");
        let second = again.data()["name"].as_str().unwrap();
        assert!(
            second.contains("(1)") && second != name,
            "{name} -> {second}"
        );
    }
    // 空文件可以上传、下载回来仍是空的。
    let empty = srv.upload(&alice, None, "empty.bin", b"");
    let r = srv.get(&alice, &format!("/files/{empty}/download"));
    assert_eq!((r.status, r.raw.len()), (200, 0));
    // 没有 file 字段：400，不留任何条目。
    let before = srv.list(&alice, None).items().len();
    let r = srv.send(
        "POST",
        "/files/upload",
        &[
            ("Authorization".into(), format!("Bearer {}", alice.token)),
            ("Content-Type".into(), format!("multipart/form-data; boundary={BOUNDARY}")),
        ],
        format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"other\"\r\n\r\nx\r\n--{BOUNDARY}--\r\n").as_bytes(),
    );
    assert_eq!(r.status, 400, "{}", r.body);
    assert_eq!(srv.list(&alice, None).items().len(), before);
}

/// 令牌伪造：过期、别的密钥、alg=none、指向不存在账户的令牌一律 401；
/// 同一个密钥签出的有效令牌则可用（证明测试能区分）。
#[test]
fn expired_foreign_keyed_none_alg_and_unknown_subject_tokens_are_rejected() {
    let (srv, alice, _bob) = two();
    let now = chrono::Utc::now().timestamp();
    let sign = |sub: &str, exp: i64, secret: &str| {
        jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
            &json!({"sub": sub, "iat": now - 10, "exp": exp}),
            &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    };
    let as_acct = |token: String| Acct {
        id: String::new(),
        token,
        email: String::new(),
    };
    let status = |token: String| srv.get(&as_acct(token), "/auth/me").status;

    assert_eq!(
        status(sign(&alice.id, now + 3600, "jwt-secret-for-api-tests")),
        200
    );
    assert_eq!(
        status(sign(&alice.id, now - 3600, "jwt-secret-for-api-tests")),
        401,
        "过期"
    );
    assert_eq!(
        status(sign(&alice.id, now + 3600, "some-other-secret")),
        401,
        "别的密钥"
    );
    assert_eq!(
        status(sign(
            &uuid::Uuid::new_v4().to_string(),
            now + 3600,
            "jwt-secret-for-api-tests"
        )),
        401,
        "不存在的账户"
    );
    let b64 = |s: &str| {
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s)
    };
    let none = format!(
        "{}.{}.",
        b64(r#"{"alg":"none","typ":"JWT"}"#),
        b64(&json!({"sub": alice.id, "iat": now, "exp": now + 3600}).to_string())
    );
    assert_eq!(status(none), 401, "alg=none");
    // 登录失败对「账户不存在」与「密码错误」给出同样的回答，不泄露账户是否存在。
    let wrong = srv.call(
        None,
        "POST",
        "/auth/login",
        Some(&json!({"identifier": alice.email, "password": "nope-nope-nope"})),
    );
    let ghost = srv.call(
        None,
        "POST",
        "/auth/login",
        Some(&json!({"identifier": "ghost@home.test", "password": "nope-nope-nope"})),
    );
    assert_eq!((wrong.status, ghost.status), (401, 401));
    assert_eq!(wrong.body, ghost.body);
}

/// Range 边界：无法解析的头与反向区间给出完整内容或 416，绝不是错误的字节；
/// 空文件、收件人通道、公开分享也遵守。
#[test]
fn malformed_ranges_never_return_wrong_bytes() {
    let (srv, alice, bob) = two();
    let content: Vec<u8> = (0u8..50).collect();
    let id = srv.upload(&alice, None, "r.bin", &content);
    let path = format!("/files/{id}/raw");
    for header in [
        "bytes=abc",
        "bytes=5-2",
        "items=0-5",
        "bytes=",
        "bytes=0-5,10-15",
        "bytes=-0",
    ] {
        let r = srv.call_h(Some(&alice), "GET", &path, None, &[("Range", header)]);
        assert!(
            (r.status == 200 && r.raw == content) || r.status == 416,
            "Range {header:?} -> {} {} 字节",
            r.status,
            r.raw.len()
        );
    }
    // 后缀比文件长：整个文件（206 或 200 均可，但字节必须是全部）。
    let r = srv.call_h(
        Some(&alice),
        "GET",
        &path,
        None,
        &[("Range", "bytes=-9999")],
    );
    assert!(
        matches!(r.status, 200 | 206) && r.raw == content,
        "{}",
        r.status
    );

    // 收件人通道也支持 Range，且给出的是对的字节。
    srv.share(&alice, &id, json!({"recipient_ids": [bob.id]}));
    let sid = srv.get(&bob, "/shares/received").items()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r = srv.call_h(
        Some(&bob),
        "GET",
        &format!("/shares/received/{sid}/files/{id}/raw"),
        None,
        &[("Range", "bytes=10-19")],
    );
    assert_eq!((r.status, r.raw.as_slice()), (206, &content[10..20]));

    // 空文件：任何 Range 都不会得到字节。
    let empty = srv.upload(&alice, None, "empty.bin", b"");
    let r = srv.call_h(
        Some(&alice),
        "GET",
        &format!("/files/{empty}/raw"),
        None,
        &[("Range", "bytes=0-0")],
    );
    assert!(r.raw.is_empty(), "空文件返回了 {} 字节", r.raw.len());
}

/// 批量下载的入参校验：空列表、非法格式、重复 id、回收站里的 id 都要得到干净的 4xx，
/// 且不会产生半截压缩包。（JSON 解析类错误不是 JSON 信封，见 plan/BUGS.md B-002。）
#[test]
fn batch_download_rejects_bad_input_cleanly() {
    let (srv, alice, _bob) = two();
    let a = srv.upload(&alice, None, "a.txt", b"aaa");
    let gone = srv.upload(&alice, None, "gone.txt", b"ggg");
    srv.delete_file(&alice, &gone);
    let post = |body: Value| srv.call(Some(&alice), "POST", "/files/download", Some(&body));

    for body in [
        json!({"ids": [], "format": "zip"}),
        json!({"ids": [a], "format": "rar"}),
        json!({"ids": ["not-a-uuid"], "format": "zip"}),
        json!({"format": "zip"}),
    ] {
        let r = post(body.clone());
        assert!(
            (400..500).contains(&r.status),
            "{body} -> {} {}",
            r.status,
            r.body
        );
    }
    assert_eq!(post(json!({"ids": [gone], "format": "zip"})).status, 404);
    assert_eq!(
        post(json!({"ids": [a, gone], "format": "tar.gz"})).status,
        404
    );
    // 重复 id 不会让同一文件在压缩包里出现两次。
    let r = post(json!({"ids": [a, a], "format": "zip"}));
    assert_eq!(r.status, 200);
    let entries = read_zip(&r.raw);
    assert_eq!(entries, [("a.txt".to_string(), b"aaa".to_vec())]);
}

/// 管理员列表不泄露密码哈希、令牌或验证码；软删除只是停用：磁盘上的字节原样保留。
#[test]
fn admin_listing_leaks_no_secrets_and_soft_delete_keeps_the_bytes() {
    let (srv, alice, _bob) = two();
    srv.upload(&alice, None, "keep.txt", b"keep-me");
    let list = srv.call_h(
        None,
        "GET",
        "/admin/users",
        None,
        &[("X-Admin-Password", ADMIN_PW)],
    );
    assert_eq!(list.status, 200);
    let text = list.body.to_string();
    for forbidden in ["argon2", "password_hash", &alice.token, "jwt-secret"] {
        assert!(!text.contains(forbidden), "管理员列表含 {forbidden}");
    }
    let before = srv.disk_files();
    assert_eq!(before.len(), 1);
    assert_eq!(
        srv.admin("/admin/users/delete", &json!({"identifier": alice.email}))
            .status,
        200
    );
    assert_eq!(srv.disk_files(), before, "软删除不得动磁盘上的文件");
    // 已停用账户的邮箱不能被别人重新注册去「接管」。
    let r = srv.call(
        None,
        "POST",
        "/auth/register",
        Some(&json!({"username": "alice2", "password": PASSWORD, "email": alice.email})),
    );
    assert_eq!(r.status, 409, "{}", r.body);
    assert_eq!(
        srv.admin("/admin/users/restore", &json!({"identifier": alice.email}))
            .status,
        200
    );
    assert_eq!(srv.get(&alice, "/files").names(), ["keep.txt"]);
}

/// SSE 覆盖每一种变更：重命名、移动、还原、永久删除都只通知所有者，并附带 usage。
#[test]
fn sse_reports_rename_move_restore_and_purge_to_the_owner_only() {
    let (srv, alice, bob) = two();
    let folder = srv.folder(&alice, None, "Dir");
    let file = srv.upload(&alice, None, "one.txt", b"12345");
    let mut a = srv.events(&alice);
    let mut b = srv.events(&bob);
    assert_eq!(a.next(Duration::from_secs(5)).unwrap().0, "usage");
    assert_eq!(b.next(Duration::from_secs(5)).unwrap().0, "usage");

    srv.call(
        Some(&alice),
        "PATCH",
        &format!("/files/{file}"),
        Some(&json!({"name": "two.txt"})),
    );
    let ev = a.wait_for("files", |d| {
        d["kind"] == "updated" || d["kind"] == "renamed"
    });
    assert!(ev.is_some(), "没收到重命名事件");
    srv.call(
        Some(&alice),
        "POST",
        &format!("/files/{file}/move"),
        Some(&json!({"target_parent_id": folder})),
    );
    assert!(
        a.wait_for("files", |d| d["file"]["id"] == file).is_some(),
        "没收到移动事件"
    );
    srv.delete_file(&alice, &file);
    let t = srv.trash_id(&alice, &file);
    assert!(a.wait_for("files", |d| d["kind"] == "deleted").is_some());
    srv.call(Some(&alice), "POST", &format!("/trash/{t}/restore"), None);
    assert!(
        a.wait_for("files", |d| d["kind"] == "created"
            || d["kind"] == "restored")
            .is_some(),
        "没收到还原事件"
    );
    assert!(
        a.wait_for("usage", |d| d["used_bytes"] == 5).is_some(),
        "还原后 usage 应回到 5"
    );
    srv.delete_file(&alice, &file);
    let t = srv.trash_id(&alice, &file);
    srv.call(Some(&alice), "DELETE", &format!("/trash/{t}"), None);
    assert!(
        a.wait_for("usage", |d| d["trashed_bytes"] == 0 && d["used_bytes"] == 0)
            .is_some(),
        "永久删除后 usage 应清零"
    );

    assert!(
        !b.sees("files", Duration::from_secs(2)),
        "事件泄露给了别的账户"
    );
}

/// 探针（B-002 的取证）：各端点遇到坏 JSON 时的响应。只断言「不是 5xx、不改状态」，
/// 响应格式的缺陷记在 plan/BUGS.md。
#[test]
fn malformed_json_bodies_are_client_errors_and_change_nothing() {
    let (srv, alice, _bob) = two();
    for (method, path) in [
        ("POST", "/folders"),
        ("POST", "/shares"),
        ("POST", "/files/download"),
        ("PATCH", "/me"),
        ("POST", "/me/password"),
    ] {
        for payload in ["{not json", "[]", "{\"name\": 5}"] {
            let r = srv.send(
                method,
                path,
                &[
                    ("Authorization".into(), format!("Bearer {}", alice.token)),
                    ("Content-Type".into(), "application/json".into()),
                ],
                payload.as_bytes(),
            );
            assert!(
                (400..500).contains(&r.status),
                "{method} {path} {payload}: {}",
                r.status
            );
        }
    }
    assert!(srv.list(&alice, None).items().is_empty());
    assert!(srv.get(&alice, "/shares").items().is_empty());
    assert_eq!(srv.get(&alice, "/auth/me").data()["username"], "alice");
}
