//! 机器人消息文案（en / 简体 / 繁體）与发往聊天提供方的字段处理（打码、清洗）。
//!
//! 发往提供方的内容仅限：申请人的显示名、**打码后**的邮箱 / 手机号、申请时间、来源 IP、
//! 管理员标签与指令回复。绝不含密码、令牌、文件名、文件内容。

use crate::config::ChatLanguage;
use crate::models::user::PROVIDER_EMAIL;

/// 邮箱打码：`alice@gmail.com` → `a***@gmail.com`。
fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            let first: String = local.chars().take(1).collect();
            format!("{first}***@{domain}")
        }
        None => "***".into(),
    }
}

/// 手机号打码：保留前 4 位（国家码）与末 3 位，其余换成 `*`。
fn mask_phone(phone: &str) -> String {
    let chars: Vec<char> = phone.chars().collect();
    if chars.len() <= 7 {
        return chars
            .iter()
            .enumerate()
            .map(|(i, c)| if i == 0 { *c } else { '*' })
            .collect();
    }
    let tail = chars.len() - 3;
    chars
        .iter()
        .enumerate()
        .map(|(i, c)| if i < 4 || i >= tail { *c } else { '*' })
        .collect()
}

/// 按配置对标识打码；`provider` 是 `email` 或 `phone`。
pub fn show_identifier(provider: &str, identifier: &str, mask: bool) -> String {
    match (mask, provider == PROVIDER_EMAIL) {
        (false, _) => identifier.to_string(),
        (true, true) => mask_email(identifier),
        (true, false) => mask_phone(identifier),
    }
}

/// 用户自填的显示名发给第三方前：去控制字符、截断。
pub fn clean_name(name: &str) -> String {
    name.chars().filter(|c| !c.is_control()).take(40).collect()
}

/// 当前时间的展示格式（UTC，精确到分钟）。
pub fn fmt_time(rfc3339: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .map(|t| {
            t.with_timezone(&chrono::Utc)
                .format("%Y-%m-%d %H:%M UTC")
                .to_string()
        })
        .unwrap_or_else(|_| rfc3339.to_string())
}

/// 一种语言的文案集。
#[derive(Debug, Clone, Copy)]
pub struct Texts(pub ChatLanguage);

impl Texts {
    fn pick(self, en: String, hans: String, hant: String) -> String {
        match self.0 {
            ChatLanguage::En => en,
            ChatLanguage::ZhHans => hans,
            ChatLanguage::ZhHant => hant,
        }
    }

    pub fn approve_button(self) -> String {
        self.pick("Approve".into(), "批准".into(), "批准".into())
    }

    pub fn reject_button(self) -> String {
        self.pick("Reject".into(), "拒绝".into(), "拒絕".into())
    }

    pub fn new_registration(self, name: &str, ident: &str, time: &str, ip: &str) -> String {
        self.pick(
            format!(
                "New registration request\nName: {name}\nContact: {ident}\nTime: {time}\nFrom: {ip}"
            ),
            format!("新的注册申请\n名称：{name}\n联系方式：{ident}\n时间：{time}\n来源：{ip}"),
            format!("新的註冊申請\n名稱：{name}\n聯絡方式：{ident}\n時間：{time}\n來源：{ip}"),
        )
    }

    pub fn burst_summary(self) -> String {
        self.pick(
            "Many registration requests arrived in a short time. Send /pending to review them."
                .into(),
            "短时间内收到多个注册申请。发送 /pending 逐个查看。".into(),
            "短時間內收到多個註冊申請。傳送 /pending 逐個查看。".into(),
        )
    }

    pub fn reminder(self, pending: usize) -> String {
        self.pick(
            format!("Reminder: {pending} registration request(s) have been waiting for more than 24 hours. Send /pending to review them."),
            format!("提醒：有 {pending} 个注册申请已等待超过 24 小时。发送 /pending 查看。"),
            format!("提醒：有 {pending} 個註冊申請已等待超過 24 小時。傳送 /pending 查看。"),
        )
    }

    /// 决定后追加在原消息末尾的一行。
    pub fn decided(self, approved: bool, by: &str, time: &str) -> String {
        match (approved, self.0) {
            (true, ChatLanguage::En) => format!("✅ Approved by {by} at {time}"),
            (false, ChatLanguage::En) => format!("❌ Rejected by {by} at {time}"),
            (true, ChatLanguage::ZhHans) => format!("✅ 已由 {by} 于 {time} 批准"),
            (false, ChatLanguage::ZhHans) => format!("❌ 已由 {by} 于 {time} 拒绝"),
            (true, ChatLanguage::ZhHant) => format!("✅ 已由 {by} 於 {time} 批准"),
            (false, ChatLanguage::ZhHant) => format!("❌ 已由 {by} 於 {time} 拒絕"),
        }
    }

    /// 点击时账户已被别人（或别的途径）决定。
    pub fn already_decided(self, line: &str) -> String {
        self.pick(
            format!("Already decided. {line}"),
            format!("已被处理。{line}"),
            format!("已被處理。{line}"),
        )
    }

    pub fn button_invalid(self) -> String {
        self.pick(
            "This button is no longer valid (used, expired or not yours). Send /pending to get fresh ones.".into(),
            "这个按钮已失效（已使用、已过期或不属于你）。发送 /pending 获取新的。".into(),
            "這個按鈕已失效（已使用、已過期或不屬於你）。傳送 /pending 取得新的。".into(),
        )
    }

    pub fn paired(self, label: &str) -> String {
        self.pick(
            format!("Paired. {label} will now receive registration requests."),
            format!("配对成功。{label} 之后会收到注册申请通知。"),
            format!("配對成功。{label} 之後會收到註冊申請通知。"),
        )
    }

    pub fn already_paired(self) -> String {
        self.pick(
            "This chat is already paired.".into(),
            "此账号已经配对。".into(),
            "此帳號已經配對。".into(),
        )
    }

    pub fn help(self) -> String {
        self.pick(
            "Cloud Home admin bot\n/pending - list registration requests\n/whoami - show who you are\n/help - this message".into(),
            "Cloud Home 管理员机器人\n/pending - 查看待处理的注册申请\n/whoami - 查看你的身份\n/help - 显示本帮助".into(),
            "Cloud Home 管理員機器人\n/pending - 查看待處理的註冊申請\n/whoami - 查看你的身份\n/help - 顯示本說明".into(),
        )
    }

    pub fn whoami(self, label: &str, provider: &str, id: &str) -> String {
        self.pick(
            format!("You are admin \"{label}\" ({provider}:{id})."),
            format!("你是管理员「{label}」（{provider}:{id}）。"),
            format!("你是管理員「{label}」（{provider}:{id}）。"),
        )
    }

    pub fn no_pending(self) -> String {
        self.pick(
            "No registration requests are waiting.".into(),
            "没有待处理的注册申请。".into(),
            "沒有待處理的註冊申請。".into(),
        )
    }

    pub fn pending_header(self, shown: usize, total: usize) -> String {
        self.pick(
            format!("{total} request(s) waiting (showing {shown}):"),
            format!("共 {total} 个待处理（显示 {shown} 个）："),
            format!("共 {total} 個待處理（顯示 {shown} 個）："),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_email_and_phone() {
        assert_eq!(mask_email("alice@gmail.com"), "a***@gmail.com");
        assert_eq!(mask_phone("+886912345123"), "+886******123");
        assert_eq!(mask_phone("+12345"), "+*****");
        assert_eq!(show_identifier("email", "bob@x.org", true), "b***@x.org");
        assert_eq!(show_identifier("email", "bob@x.org", false), "bob@x.org");
        assert!(!show_identifier("phone", "+886912345123", true).contains("91234"));
    }

    #[test]
    fn names_are_cleaned_before_leaving_the_device() {
        assert_eq!(clean_name("a\nb\u{7}c"), "abc");
        assert_eq!(clean_name(&"x".repeat(100)).chars().count(), 40);
    }

    #[test]
    fn decided_line_is_localized() {
        assert!(
            Texts(ChatLanguage::En)
                .decided(true, "Ann", "T")
                .starts_with("✅ Approved by Ann")
        );
        assert!(
            Texts(ChatLanguage::ZhHans)
                .decided(false, "Ann", "T")
                .contains("拒绝")
        );
        assert!(
            Texts(ChatLanguage::ZhHant)
                .decided(false, "Ann", "T")
                .contains("拒絕")
        );
    }
}
