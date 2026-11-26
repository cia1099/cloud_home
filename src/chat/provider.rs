//! 聊天提供方抽象：归一化的消息 / 更新类型与 `ChatProvider` trait。
//!
//! 核心逻辑（配对、通知、按钮审批）只依赖这里的类型，不认识具体的聊天应用；
//! 换一个提供方只需再实现一次 trait。

use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

/// 提供方调用失败。文本里不得出现机器人令牌（实现者负责脱敏）。
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ChatError(pub String);

/// trait 要 dyn 兼容（`Arc<dyn ChatProvider>`），所以异步方法手写成装箱 future。
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = Result<T, ChatError>> + Send + 'a>>;

/// 消息上的一个按钮。`token` 是服务器端映射表里的不透明随机串，不含任何业务信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Button {
    pub label: String,
    pub token: String,
}

/// 发给聊天的消息：纯文本 + 可选按钮（一行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub text: String,
    pub buttons: Vec<Button>,
}

impl Message {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            buttons: Vec::new(),
        }
    }
}

/// 已发出的消息的引用，用于事后编辑。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRef {
    pub chat: String,
    pub id: String,
}

/// 更新的发送者。`id` 是提供方的不可变数字 id（不是用户名）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sender {
    pub id: String,
    /// 展示用名称（仅用于配对时给管理员起标签）。
    pub label: String,
    /// 回复要发往的会话。
    pub chat: String,
    /// 是否与机器人的私聊（群聊里的指令与按钮一律不处理）。
    pub private: bool,
}

/// 归一化的入站更新。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {
    Command {
        from: Sender,
        /// 小写、不含 `/` 与 `@bot` 后缀。
        name: String,
        args: String,
    },
    ButtonPress {
        from: Sender,
        message: MessageRef,
        token: String,
        /// 提供方要求应答按钮点击时用的句柄（Telegram 的 callback_query id）。
        ack: String,
    },
}

pub trait ChatProvider: Send + Sync {
    fn send<'a>(&'a self, chat: &'a str, msg: &'a Message) -> BoxFut<'a, MessageRef>;
    /// 编辑已发出的消息；`msg.buttons` 为空即移除按钮。
    fn edit<'a>(&'a self, target: &'a MessageRef, msg: &'a Message) -> BoxFut<'a, ()>;
    /// 应答一次按钮点击（让客户端结束「加载中」状态）。
    fn ack_button<'a>(&'a self, ack: &'a str) -> BoxFut<'a, ()>;
    /// 拉取新的更新（长轮询实现会阻塞到有更新或超时，超时返回空列表）。
    fn next_updates(&self) -> BoxFut<'_, Vec<Update>>;
}

/// 内存里的假提供方：整条聊天流程无需网络即可测试。
#[cfg(test)]
pub use fake::FakeProvider;

#[cfg(test)]
mod fake {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct Inner {
        sent: Vec<(String, Message, MessageRef)>,
        edits: Vec<(MessageRef, Message)>,
        acks: Vec<String>,
        updates: VecDeque<Update>,
        down: bool,
        next_id: u64,
    }

    #[derive(Default)]
    pub struct FakeProvider(Mutex<Inner>);

    impl FakeProvider {
        pub fn push_update(&self, u: Update) {
            self.0.lock().unwrap().updates.push_back(u);
        }

        /// 模拟聊天服务不可达：send / edit 全部失败。
        pub fn set_down(&self, down: bool) {
            self.0.lock().unwrap().down = down;
        }

        /// 已发出的 (会话, 消息, 引用)。
        pub fn sent(&self) -> Vec<(String, Message, MessageRef)> {
            self.0.lock().unwrap().sent.clone()
        }

        pub fn edits(&self) -> Vec<(MessageRef, Message)> {
            self.0.lock().unwrap().edits.clone()
        }

        pub fn acks(&self) -> Vec<String> {
            self.0.lock().unwrap().acks.clone()
        }
    }

    impl ChatProvider for FakeProvider {
        fn send<'a>(&'a self, chat: &'a str, msg: &'a Message) -> BoxFut<'a, MessageRef> {
            Box::pin(async move {
                let mut g = self.0.lock().unwrap();
                if g.down {
                    return Err(ChatError("fake provider is down".into()));
                }
                g.next_id += 1;
                let r = MessageRef {
                    chat: chat.into(),
                    id: g.next_id.to_string(),
                };
                g.sent.push((chat.into(), msg.clone(), r.clone()));
                Ok(r)
            })
        }

        fn edit<'a>(&'a self, target: &'a MessageRef, msg: &'a Message) -> BoxFut<'a, ()> {
            Box::pin(async move {
                let mut g = self.0.lock().unwrap();
                if g.down {
                    return Err(ChatError("fake provider is down".into()));
                }
                g.edits.push((target.clone(), msg.clone()));
                Ok(())
            })
        }

        fn ack_button<'a>(&'a self, ack: &'a str) -> BoxFut<'a, ()> {
            Box::pin(async move {
                self.0.lock().unwrap().acks.push(ack.into());
                Ok(())
            })
        }

        fn next_updates(&self) -> BoxFut<'_, Vec<Update>> {
            Box::pin(async move { Ok(self.0.lock().unwrap().updates.drain(..).collect()) })
        }
    }
}
