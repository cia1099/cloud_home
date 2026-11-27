//! 站内定向分享（R-071）的端到端测试：启动真实的 `cloud_home` 二进制
//! （临时数据盘、一次性 JWT 密钥、空闲端口、临时日志目录），只通过 HTTP 驱动。
//! 重点是账户隔离：不是收件人的账户对分享内容一律 404。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const ADMIN_PW: &str = "adm1n-secret-pw";
const PASSWORD: &str = "correct-horse-battery";

struct Resp {
    status: u16,
    body: Value,
    raw: Vec<u8>,
}

struct Server {
    child: Child,
    port: u16,
    dir: PathBuf,
}

/// 一个已登录的测试账户。
struct Acct {
    id: String,
    token: String,
}

impl Server {
    fn start() -> Self {
        let dir = std::env::temp_dir().join(format!("ch-share-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let out = std::fs::File::create(dir.join("stdout.log")).unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_cloud_home"))
            .current_dir(&dir)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("SERVER_HOST", "127.0.0.1")
            .env("SERVER_PORT", port.to_string())
            .env("JWT_SECRET", "jwt-secret-for-share-tests")
            .env("ADMIN_PASSWORD", ADMIN_PW)
            .env("DEV_DRIVE_PATH", dir.join("drive"))
            .env("VOLUMES_WATCH_PATH", dir.join("no-volumes"))
            .env("LOG_DIR", dir.join("logs"))
            .env("UPDATE_ROOT", dir.join("update"))
            .env("CHAT_PROVIDER", "none")
            .stdout(Stdio::from(out.try_clone().unwrap()))
            .stderr(Stdio::from(out))
            .spawn()
            .unwrap();
        let srv = Self { child, port, dir };
        let deadline = Instant::now() + Duration::from_secs(30);
        while TcpStream::connect(("127.0.0.1", srv.port)).is_err() {
            assert!(Instant::now() < deadline, "服务未在 30 秒内就绪");
            std::thread::sleep(Duration::from_millis(50));
        }
        srv
    }

    fn send(&self, method: &str, path: &str, headers: &[(String, String)], body: &[u8]) -> Resp {
        let mut s = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        let mut req = format!("{method} /api/v1{path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
        s.write_all(req.as_bytes()).unwrap();
        s.write_all(body).unwrap();
        let mut raw = Vec::new();
        s.read_to_end(&mut raw).unwrap();
        let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&raw[..split]).into_owned();
        let body = raw[split + 4..].to_vec();
        let status = head.lines().next().unwrap().split_whitespace().nth(1).unwrap().parse().unwrap();
        Resp {
            status,
            body: serde_json::from_slice(&body).unwrap_or(Value::Null),
            raw: body,
        }
    }

    fn call(&self, who: Option<&Acct>, method: &str, path: &str, body: Option<&Value>) -> Resp {
        self.call_h(who, method, path, body, &[])
    }

    fn call_h(
        &self,
        who: Option<&Acct>,
        method: &str,
        path: &str,
        body: Option<&Value>,
        extra: &[(&str, &str)],
    ) -> Resp {
        let mut h: Vec<(String, String)> = extra.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        if let Some(a) = who {
            h.push(("Authorization".into(), format!("Bearer {}", a.token)));
        }
        let payload = body.map(Value::to_string).unwrap_or_default();
        if body.is_some() {
            h.push(("Content-Type".into(), "application/json".into()));
        }
        self.send(method, path, &h, payload.as_bytes())
    }

    /// 注册并批准、登录。第一个本机注册的账户自动批准，其余走管理端批准。
    fn account(&self, name: &str) -> Acct {
        let email = format!("{name}@home.test");
        self.send(
            "POST",
            "/auth/register",
            &[("Content-Type".into(), "application/json".into())],
            json!({"username": name, "password": PASSWORD, "email": email}).to_string().as_bytes(),
        );
        self.approve(&email);
        self.login(name)
    }

    fn approve(&self, email: &str) {
        let r = self.call_h(
            None,
            "POST",
            "/admin/users/approve",
            Some(&json!({"identifier": email})),
            &[("X-Admin-Password", ADMIN_PW)],
        );
        assert!(r.status == 200 || r.status == 409, "approve: {} {}", r.status, r.body);
    }

    fn login(&self, name: &str) -> Acct {
        let r = self.call(
            None,
            "POST",
            "/auth/login",
            Some(&json!({"identifier": format!("{name}@home.test"), "password": PASSWORD})),
        );
        assert_eq!(r.status, 200, "login {name}: {}", r.body);
        let token = r.body["data"]["token"].as_str().unwrap().to_string();
        let acct = Acct { id: String::new(), token };
        let me = self.call(Some(&acct), "GET", "/auth/me", None);
        Acct { id: me.body["data"]["id"].as_str().unwrap().to_string(), ..acct }
    }

    fn upload(&self, who: &Acct, parent: Option<&str>, name: &str, content: &[u8]) -> String {
        let b = "XBOUNDARYX";
        let mut body = Vec::new();
        if let Some(p) = parent {
            body.extend(format!("--{b}\r\nContent-Disposition: form-data; name=\"parent_id\"\r\n\r\n{p}\r\n").as_bytes());
        }
        body.extend(
            format!("--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n")
                .as_bytes(),
        );
        body.extend(content);
        body.extend(format!("\r\n--{b}--\r\n").as_bytes());
        let r = self.send(
            "POST",
            "/files/upload",
            &[
                ("Authorization".into(), format!("Bearer {}", who.token)),
                ("Content-Type".into(), format!("multipart/form-data; boundary={b}")),
            ],
            &body,
        );
        assert_eq!(r.status, 201, "upload: {}", r.body);
        r.body["data"]["id"].as_str().unwrap().to_string()
    }

    fn folder(&self, who: &Acct, parent: Option<&str>, name: &str) -> String {
        let r = self.call(Some(who), "POST", "/folders", Some(&json!({"name": name, "parent_id": parent})));
        assert_eq!(r.status, 201, "folder: {}", r.body);
        r.body["data"]["id"].as_str().unwrap().to_string()
    }

    fn share(&self, who: &Acct, file_id: &str, extra: Value) -> Value {
        let mut body = json!({"file_id": file_id});
        body.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let r = self.call(Some(who), "POST", "/shares", Some(&body));
        assert_eq!(r.status, 201, "share: {}", r.body);
        r.body["data"].clone()
    }

    fn received(&self, who: &Acct) -> Vec<Value> {
        let r = self.call(Some(who), "GET", "/shares/received", None);
        assert_eq!(r.status, 200);
        r.body["data"]["items"].as_array().unwrap().clone()
    }

    async fn db(&self) -> sqlx::SqlitePool {
        sqlx::SqlitePool::connect(&format!(
            "sqlite://{}",
            self.dir.join("drive/cloud_home_data/cloud_home.db").display()
        ))
        .await
        .unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// owner（首个本机注册，自动批准）、ming、rose、eve 四个已批准账户。
fn family() -> (Server, Acct, Acct, Acct, Acct) {
    let srv = Server::start();
    let owner = srv.account("owner");
    let ming = srv.account("ming");
    let rose = srv.account("rose");
    let eve = srv.account("eve");
    (srv, owner, ming, rose, eve)
}

fn path_of(share: &str, file: &str, tail: &str) -> String {
    format!("/shares/received/{share}/files/{file}{tail}")
}

#[test]
fn recipient_can_view_stream_and_download_but_a_stranger_gets_404_everywhere() {
    let (srv, owner, ming, _rose, eve) = family();
    let file = srv.upload(&owner, None, "note.txt", b"hello shared world");
    let share = srv.share(&owner, &file, json!({"recipient_ids": [ming.id]}));
    let sid = share["id"].as_str().unwrap();

    // 定向分享没有公开令牌；owner 的列表带收件人。
    assert!(share["token"].is_null() && share["url"].is_null());
    assert_eq!(share["recipients"][0]["id"], ming.id);
    assert_eq!(share["recipients"][0]["display_name"], "ming");
    assert!(share["recipients"][0].get("email").is_none());
    let listed = srv.call(Some(&owner), "GET", "/shares", None);
    assert_eq!(listed.body["data"]["items"][0]["recipients"][0]["display_name"], "ming");

    // 收件人：列表、元数据、Range 流、下载。
    let items = srv.received(&ming);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "note.txt");
    assert_eq!(items[0]["owner"]["display_name"], "owner");
    assert_eq!(items[0]["can_download"], true);
    assert!(items[0]["seen_at"].is_null());
    assert!(items[0]["child_count"].is_null());
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(sid, &file, ""), None).status, 200);
    let part = srv.call_h(Some(&ming), "GET", &path_of(sid, &file, "/raw"), None, &[("Range", "bytes=0-4")]);
    assert_eq!((part.status, part.raw.as_slice()), (206, b"hello".as_slice()));
    let dl = srv.call(Some(&ming), "GET", &path_of(sid, &file, "/download"), None);
    assert_eq!((dl.status, dl.raw.as_slice()), (200, b"hello shared world".as_slice()));

    // 已读标记只记第一次。
    assert_eq!(srv.call(Some(&ming), "POST", &format!("/shares/{sid}/seen"), None).status, 200);
    let first = srv.received(&ming)[0]["seen_at"].clone();
    assert!(first.is_string());
    srv.call(Some(&ming), "POST", &format!("/shares/{sid}/seen"), None);
    assert_eq!(srv.received(&ming)[0]["seen_at"], first);

    // 非收件人（含无关账户 eve）：全部 404，列表为空，也不能标记已读或改收件人。
    assert!(srv.received(&eve).is_empty());
    for tail in ["", "/raw", "/download", "/thumbnail"] {
        assert_eq!(srv.call(Some(&eve), "GET", &path_of(sid, &file, tail), None).status, 404, "{tail}");
    }
    assert_eq!(srv.call(Some(&eve), "GET", &format!("/shares/received/{sid}/files"), None).status, 404);
    assert_eq!(srv.call(Some(&eve), "POST", &format!("/shares/{sid}/seen"), None).status, 404);
    assert_eq!(
        srv.call(Some(&eve), "PATCH", &format!("/shares/{sid}"), Some(&json!({"recipient_ids": [eve.id]}))).status,
        404
    );
    assert_eq!(srv.call(Some(&eve), "DELETE", &format!("/shares/{sid}"), None).status, 404);
    // 收件人自己也不能撤销、也读不了 owner 的原始端点。
    assert_eq!(srv.call(Some(&ming), "DELETE", &format!("/shares/{sid}"), None).status, 404);
    assert_eq!(srv.call(Some(&ming), "GET", &format!("/files/{file}"), None).status, 404);
    assert!(srv.call(Some(&ming), "GET", "/shares", None).body["data"]["items"].as_array().unwrap().is_empty());
    // 没登录：401。
    assert_eq!(srv.call(None, "GET", "/shares/received", None).status, 401);
}

#[test]
fn directed_share_is_never_served_through_the_public_token_endpoint() {
    let (srv, owner, ming, ..) = family();
    let file = srv.upload(&owner, None, "a.txt", b"x");
    let sid = srv.share(&owner, &file, json!({"recipient_ids": [ming.id]}))["id"].as_str().unwrap().to_string();
    let public = srv.share(&owner, &file, json!({}));
    let ptoken = public["token"].as_str().unwrap();
    assert!(public["recipients"].as_array().unwrap().is_empty());
    assert_eq!(srv.call(None, "GET", &format!("/public/shares/{ptoken}"), None).status, 200);

    let db = tokio::runtime::Runtime::new().unwrap().block_on(async {
        let pool = srv.db().await;
        let (token,): (String,) = sqlx::query_as("SELECT token FROM shares WHERE id = ?")
            .bind(&sid)
            .fetch_one(&pool)
            .await
            .unwrap();
        token
    });
    assert_eq!(srv.call(None, "GET", &format!("/public/shares/{db}"), None).status, 404);
    assert_eq!(srv.call(None, "GET", &format!("/public/shares/{db}/download"), None).status, 404);
}

#[test]
fn folder_share_is_limited_to_its_subtree() {
    let (srv, owner, ming, _rose, eve) = family();
    let album = srv.folder(&owner, None, "Album");
    let inner = srv.folder(&owner, Some(&album), "Inner");
    let deep = srv.upload(&owner, Some(&inner), "deep.txt", b"deep");
    let top = srv.upload(&owner, Some(&album), "top.txt", b"top");
    let outside = srv.upload(&owner, None, "secret.txt", b"secret");
    let sid = srv.share(&owner, &album, json!({"recipient_ids": [ming.id]}))["id"]
        .as_str()
        .unwrap()
        .to_string();

    let item = &srv.received(&ming)[0];
    assert_eq!(item["file_type"], "folder");
    assert_eq!(item["child_count"], 2);

    let root = srv.call(Some(&ming), "GET", &format!("/shares/received/{sid}/files"), None);
    assert_eq!(root.status, 200);
    let names: Vec<&str> = root.body["data"]["items"].as_array().unwrap().iter().map(|i| i["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Inner", "top.txt"]);
    let sub = srv.call(Some(&ming), "GET", &format!("/shares/received/{sid}/files?parent_id={inner}"), None);
    assert_eq!(sub.body["data"]["items"][0]["name"], "deep.txt");
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&sid, &deep, "/download"), None).raw, b"deep");
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&sid, &top, "/raw"), None).raw, b"top");

    // 子树之外（含 owner 的另一个文件）404；列出子树之外的目录也 404；对文件列目录 400。
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&sid, &outside, ""), None).status, 404);
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&sid, &outside, "/download"), None).status, 404);
    assert_eq!(srv.call(Some(&ming), "GET", &format!("/shares/received/{sid}/files?parent_id={outside}"), None).status, 404);
    assert_eq!(srv.call(Some(&ming), "GET", &format!("/shares/received/{sid}/files?parent_id={top}"), None).status, 400);
    // 文件夹本身不能当文件下载。
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&sid, &album, "/download"), None).status, 400);
    // 别人的 id 拼进别人的分享：eve 什么都碰不到。
    assert_eq!(srv.call(Some(&eve), "GET", &path_of(&sid, &deep, ""), None).status, 404);
    // 收件人自己的文件 id 拼进来也不行（owner 以外账户的文件不在被分享子树内）。
    let mine = srv.upload(&ming, None, "mine.txt", b"mine");
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&sid, &mine, ""), None).status, 404);
    // 子树里的文件被 owner 删除后不再可见。
    assert_eq!(srv.call(Some(&owner), "DELETE", &format!("/files/{deep}"), None).status, 200);
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&sid, &deep, ""), None).status, 404);
}

#[test]
fn can_download_false_still_allows_preview_but_not_download() {
    let (srv, owner, ming, ..) = family();
    let file = srv.upload(&owner, None, "v.txt", b"view only");
    let sid = srv.share(&owner, &file, json!({"recipient_ids": [ming.id], "can_download": false}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(srv.received(&ming)[0]["can_download"], false);
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&sid, &file, "/raw"), None).status, 200);
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&sid, &file, "/download"), None).status, 403);
}

#[test]
fn revoke_edit_expiry_and_trash_cut_access_immediately() {
    let (srv, owner, ming, rose, _eve) = family();
    let file = srv.upload(&owner, None, "a.txt", b"aaa");

    // 撤销。
    let s1 = srv.share(&owner, &file, json!({"recipient_ids": [ming.id]}))["id"].as_str().unwrap().to_string();
    assert_eq!(srv.received(&ming).len(), 1);
    assert_eq!(srv.call(Some(&owner), "DELETE", &format!("/shares/{s1}"), None).status, 200);
    assert!(srv.received(&ming).is_empty());
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&s1, &file, ""), None).status, 404);
    // 撤销后不能再改。
    let r = srv.call(Some(&owner), "PATCH", &format!("/shares/{s1}"), Some(&json!({"recipient_ids": [ming.id]})));
    assert_eq!(r.status, 404);

    // 编辑收件人：整组替换，ming 出局、rose 进来。
    let s2 = srv.share(&owner, &file, json!({"recipient_ids": [ming.id]}))["id"].as_str().unwrap().to_string();
    let r = srv.call(Some(&owner), "PATCH", &format!("/shares/{s2}"), Some(&json!({"recipient_ids": [rose.id]})));
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(r.body["data"]["recipients"][0]["id"], rose.id);
    assert!(srv.received(&ming).is_empty());
    assert_eq!(srv.received(&rose).len(), 1);
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&s2, &file, ""), None).status, 404);
    assert_eq!(srv.call(Some(&rose), "GET", &path_of(&s2, &file, ""), None).status, 200);
    // 不能清空（否则会变回无限制），也不能把公开链接改成定向。
    let r = srv.call(Some(&owner), "PATCH", &format!("/shares/{s2}"), Some(&json!({"recipient_ids": []})));
    assert_eq!(r.status, 400);
    let public = srv.share(&owner, &file, json!({}))["id"].as_str().unwrap().to_string();
    let r = srv.call(Some(&owner), "PATCH", &format!("/shares/{public}"), Some(&json!({"recipient_ids": [rose.id]})));
    assert_eq!(r.status, 400);

    // 已过期的分享不出现、不可访问。
    let s3 = srv.share(&owner, &file, json!({"recipient_ids": [ming.id], "expires_at": "2000-01-01T00:00:00Z"}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(srv.received(&ming).is_empty());
    assert_eq!(srv.call(Some(&ming), "GET", &path_of(&s3, &file, ""), None).status, 404);

    // 被分享的文件进回收站后，收件人看不到。
    let s4 = srv.share(&owner, &file, json!({"recipient_ids": [rose.id]}))["id"].as_str().unwrap().to_string();
    assert_eq!(srv.received(&rose).len(), 2);
    assert_eq!(srv.call(Some(&owner), "DELETE", &format!("/files/{file}"), None).status, 200);
    assert!(srv.received(&rose).is_empty());
    assert_eq!(srv.call(Some(&rose), "GET", &path_of(&s4, &file, "/raw"), None).status, 404);
}

#[test]
fn recipients_must_be_other_approved_active_users() {
    let (srv, owner, ming, ..) = family();
    let file = srv.upload(&owner, None, "a.txt", b"a");
    // 待批准的账户。
    srv.send(
        "POST",
        "/auth/register",
        &[("Content-Type".into(), "application/json".into())],
        json!({"username": "pending", "password": PASSWORD, "email": "pending@home.test"}).to_string().as_bytes(),
    );
    let dir = srv.call(Some(&ming), "GET", "/users/directory", None);
    let people = dir.body["data"]["items"].as_array().unwrap();
    let pending_id = {
        let r = srv.call_h(None, "GET", "/admin/users", None, &[("X-Admin-Password", ADMIN_PW)]);
        r.body["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|u| u["username"] == "pending")
            .map(|u| u["id"].as_str().unwrap().to_string())
            .unwrap()
    };
    let bad: [Value; 4] = [
        json!([owner.id.clone()]),
        json!(["no-such-user"]),
        json!([pending_id.clone()]),
        json!([ming.id.clone(), "no-such-user"]),
    ];
    for ids in bad {
        let r = srv.call(Some(&owner), "POST", "/shares", Some(&json!({"file_id": file, "recipient_ids": ids})));
        assert_eq!(r.status, 400, "{ids}: {}", r.body);
    }
    // 目录：只有 id + display_name，没有自己、没有 pending。
    assert!(people.iter().all(|p| p.as_object().unwrap().len() == 2));
    let names: Vec<&str> = people.iter().map(|p| p["display_name"].as_str().unwrap()).collect();
    assert!(names.contains(&"owner") && names.contains(&"rose") && !names.contains(&"ming") && !names.contains(&"pending"));
    assert_eq!(srv.call(None, "GET", "/users/directory", None).status, 401);
    // 别人的文件不能被分享出去。
    let r = srv.call(Some(&ming), "POST", "/shares", Some(&json!({"file_id": file, "recipient_ids": [owner.id]})));
    assert_eq!(r.status, 404);
}

/// 读 SSE 流直到出现某段文本（或超时）。
fn read_until(s: &mut TcpStream, buf: &mut String, needle: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut chunk = [0u8; 4096];
    while !buf.contains(needle) {
        assert!(Instant::now() < deadline, "SSE 里没等到 {needle}：\n{buf}");
        if let Ok(n) = s.read(&mut chunk) {
            buf.push_str(&String::from_utf8_lossy(&chunk[..n]));
        }
    }
}

#[test]
fn sse_announces_received_and_revoked_only_to_the_recipient() {
    let (srv, owner, ming, _rose, eve) = family();
    let file = srv.upload(&owner, None, "a.txt", b"a");
    let open = |who: &Acct| {
        let mut s = TcpStream::connect(("127.0.0.1", srv.port)).unwrap();
        s.set_read_timeout(Some(Duration::from_millis(300))).unwrap();
        write!(
            s,
            "GET /api/v1/events/stream HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\n\r\n",
            who.token
        )
        .unwrap();
        let mut buf = String::new();
        read_until(&mut s, &mut buf, "event: usage");
        (s, buf)
    };
    let (mut ming_s, mut ming_buf) = open(&ming);
    let (mut eve_s, mut eve_buf) = open(&eve);

    let sid = srv.share(&owner, &file, json!({"recipient_ids": [ming.id]}))["id"].as_str().unwrap().to_string();
    read_until(&mut ming_s, &mut ming_buf, "event: share.received");
    assert!(ming_buf.contains(&sid));

    srv.call(Some(&owner), "DELETE", &format!("/shares/{sid}"), None);
    read_until(&mut ming_s, &mut ming_buf, "event: share.revoked");

    // eve 的流里没有任何分享事件（让它再多读一会儿）。
    let mut chunk = [0u8; 4096];
    for _ in 0..5 {
        if let Ok(n) = eve_s.read(&mut chunk) {
            eve_buf.push_str(&String::from_utf8_lossy(&chunk[..n]));
        }
    }
    assert!(!eve_buf.contains("share."), "{eve_buf}");
}
