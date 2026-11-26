//! Telegram Bot API 提供方：HTTPS 长轮询，只发起出站连接（不开入站端口、不用 webhook）。
//!
//! 令牌在 URL 里，所以：不 `derive(Debug)`；reqwest 的错误一律先 `without_url()`
//! 再转成文本；任何日志、错误信息都不含令牌。

use std::sync::Mutex;
use std::time::Duration;

use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::provider::{BoxFut, ChatError, ChatProvider, Message, MessageRef, Sender, Update};

const API: &str = "https://api.telegram.org";
/// 服务端长轮询等待秒数；客户端超时要比它长。
const POLL_SECS: u64 = 30;

pub struct TelegramProvider {
    http: reqwest::Client,
    /// `https://api.telegram.org/bot<token>`——含密钥，永不输出。
    base: String,
    /// 下一次 getUpdates 的 offset（已确认的最大 update_id + 1）。
    offset: Mutex<i64>,
}

impl TelegramProvider {
    pub fn new(token: &str) -> anyhow::Result<Self> {
        // reqwest 用 rustls-no-provider：这里显式装 ring。已装过则忽略。
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder()
            .user_agent(format!("cloud_home/{}", crate::version::VERSION))
            .connect_timeout(Duration::from_secs(15))
            .build()?;
        Ok(Self {
            http,
            base: format!("{API}/bot{token}"),
            offset: Mutex::new(0),
        })
    }

    async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        body: Value,
        timeout: Duration,
    ) -> Result<T, ChatError> {
        let resp = self
            .http
            .post(format!("{}/{method}", self.base))
            .header(CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| ChatError(format!("telegram {method}: {}", e.without_url())))?;
        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ChatError(format!("telegram {method}: {}", e.without_url())))?;
        let env: Envelope<T> = serde_json::from_slice(&bytes)
            .map_err(|_| ChatError(format!("telegram {method}: HTTP {status}, 响应无法解析")))?;
        match (env.ok, env.result) {
            (true, Some(r)) => Ok(r),
            _ => Err(ChatError(format!(
                "telegram {method}: {}",
                env.description.unwrap_or_else(|| format!("HTTP {status}"))
            ))),
        }
    }
}

const CALL_TIMEOUT: Duration = Duration::from_secs(15);

/// inline keyboard：一行按钮；空列表即移除键盘。
fn markup(msg: &Message) -> Value {
    let row: Vec<Value> = msg
        .buttons
        .iter()
        .map(|b| json!({ "text": b.label, "callback_data": b.token }))
        .collect();
    if row.is_empty() {
        json!({ "inline_keyboard": [] })
    } else {
        json!({ "inline_keyboard": [row] })
    }
}

impl ChatProvider for TelegramProvider {
    fn send<'a>(&'a self, chat: &'a str, msg: &'a Message) -> BoxFut<'a, MessageRef> {
        Box::pin(async move {
            let body = json!({
                "chat_id": chat,
                "text": msg.text,
                "reply_markup": markup(msg),
            });
            let sent: RawMessage = self.call("sendMessage", body, CALL_TIMEOUT).await?;
            Ok(MessageRef {
                chat: chat.to_string(),
                id: sent.message_id.to_string(),
            })
        })
    }

    fn edit<'a>(&'a self, target: &'a MessageRef, msg: &'a Message) -> BoxFut<'a, ()> {
        Box::pin(async move {
            let body = json!({
                "chat_id": target.chat,
                "message_id": target.id.parse::<i64>().unwrap_or_default(),
                "text": msg.text,
                "reply_markup": markup(msg),
            });
            let _: Value = self.call("editMessageText", body, CALL_TIMEOUT).await?;
            Ok(())
        })
    }

    fn ack_button<'a>(&'a self, ack: &'a str) -> BoxFut<'a, ()> {
        Box::pin(async move {
            let body = json!({ "callback_query_id": ack });
            let _: Value = self.call("answerCallbackQuery", body, CALL_TIMEOUT).await?;
            Ok(())
        })
    }

    fn next_updates(&self) -> BoxFut<'_, Vec<Update>> {
        Box::pin(async move {
            let offset = *self.offset.lock().unwrap();
            let body = json!({
                "offset": offset,
                "timeout": POLL_SECS,
                "allowed_updates": ["message", "callback_query"],
            });
            let raw: Vec<RawUpdate> = self
                .call("getUpdates", body, Duration::from_secs(POLL_SECS + 10))
                .await?;
            if let Some(max) = raw.iter().map(|u| u.update_id).max() {
                *self.offset.lock().unwrap() = max + 1;
            }
            Ok(raw.into_iter().filter_map(RawUpdate::into_update).collect())
        })
    }
}

#[derive(Deserialize)]
struct Envelope<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct RawUpdate {
    update_id: i64,
    message: Option<RawMessage>,
    callback_query: Option<RawCallback>,
}

#[derive(Deserialize)]
struct RawMessage {
    message_id: i64,
    from: Option<RawUser>,
    chat: Option<RawChat>,
    text: Option<String>,
}

#[derive(Deserialize)]
struct RawUser {
    id: i64,
    first_name: Option<String>,
    username: Option<String>,
}

#[derive(Deserialize)]
struct RawChat {
    id: i64,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct RawCallback {
    id: String,
    from: RawUser,
    message: Option<RawMessage>,
    data: Option<String>,
}

impl RawUser {
    fn label(&self) -> String {
        self.first_name
            .clone()
            .or_else(|| self.username.clone())
            .unwrap_or_else(|| self.id.to_string())
    }
}

impl RawUpdate {
    /// 只认文本指令与按钮点击；其余（贴图、普通文字、群事件……）一律丢弃。
    fn into_update(self) -> Option<Update> {
        if let Some(cb) = self.callback_query {
            let msg = cb.message?;
            let chat = msg.chat?;
            return Some(Update::ButtonPress {
                from: Sender {
                    id: cb.from.id.to_string(),
                    label: cb.from.label(),
                    chat: chat.id.to_string(),
                    private: chat.kind == "private",
                },
                message: MessageRef {
                    chat: chat.id.to_string(),
                    id: msg.message_id.to_string(),
                },
                token: cb.data?,
                ack: cb.id,
            });
        }
        let msg = self.message?;
        let (name, args) = parse_command(msg.text.as_deref()?)?;
        let from = msg.from?;
        let chat = msg.chat?;
        Some(Update::Command {
            from: Sender {
                id: from.id.to_string(),
                label: from.label(),
                chat: chat.id.to_string(),
                private: chat.kind == "private",
            },
            name,
            args,
        })
    }
}

/// `/pair@MyBot  ABCD` → `("pair", "ABCD")`；不是指令返回 None。
fn parse_command(text: &str) -> Option<(String, String)> {
    let rest = text.trim().strip_prefix('/')?;
    let (head, args) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let name = head.split('@').next().unwrap_or("").to_ascii_lowercase();
    if name.is_empty() {
        return None;
    }
    Some((name, args.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::provider::Button;

    #[test]
    fn parses_commands_with_bot_suffix_and_args() {
        assert_eq!(
            parse_command("/pair@MyBot  ab12 "),
            Some(("pair".into(), "ab12".into()))
        );
        assert_eq!(
            parse_command("/Start"),
            Some(("start".into(), String::new()))
        );
        assert_eq!(parse_command("hello"), None);
        assert_eq!(parse_command("/"), None);
    }

    fn updates(json: &str) -> Vec<Update> {
        let raw: Vec<RawUpdate> = serde_json::from_str(json).unwrap();
        raw.into_iter().filter_map(RawUpdate::into_update).collect()
    }

    #[test]
    fn normalizes_message_and_callback_updates() {
        let got = updates(
            r#"[
              {"update_id":1,"message":{"message_id":5,"from":{"id":42,"first_name":"Ann"},
               "chat":{"id":42,"type":"private"},"text":"/pending"}},
              {"update_id":2,"callback_query":{"id":"cb1","from":{"id":42,"first_name":"Ann"},
               "message":{"message_id":9,"chat":{"id":42,"type":"private"}},"data":"tok"}},
              {"update_id":3,"message":{"message_id":6,"from":{"id":7},"chat":{"id":-100,"type":"group"},"text":"/help"}},
              {"update_id":4,"message":{"message_id":7,"from":{"id":7},"chat":{"id":7,"type":"private"},"text":"plain text"}}
            ]"#,
        );
        assert_eq!(got.len(), 3, "普通文字被丢弃");
        assert!(matches!(&got[0], Update::Command { from, name, .. }
            if from.id == "42" && from.private && name == "pending"));
        assert!(
            matches!(&got[1], Update::ButtonPress { token, ack, message, .. }
            if token == "tok" && ack == "cb1" && message.id == "9")
        );
        assert!(matches!(&got[2], Update::Command { from, .. } if !from.private));
    }

    #[test]
    fn markup_is_empty_without_buttons() {
        assert_eq!(markup(&Message::text("x")), json!({"inline_keyboard": []}));
        let m = Message {
            text: "x".into(),
            buttons: vec![Button {
                label: "Go".into(),
                token: "t".into(),
            }],
        };
        assert_eq!(
            markup(&m),
            json!({"inline_keyboard": [[{"text": "Go", "callback_data": "t"}]]})
        );
    }
}
