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
            "Cloud Home admin bot\n/pending - list registration requests\n/users [pending|approved|rejected] - list accounts\n/status - drive and account summary\n/delete <account> - delete an account (asks to confirm)\n/restore <account> - restore a deleted account\n/purge <account> - empty an account's trash for good (asks to confirm)\n/confirm <code> - confirm the last delete or purge\n/cancel - cancel it\n/whoami - show who you are\n/help - this message".into(),
            "Cloud Home 管理员机器人\n/pending - 查看待处理的注册申请\n/users [pending|approved|rejected] - 列出账户\n/status - 硬盘与账户概况\n/delete <账户> - 删除账户（需确认）\n/restore <账户> - 还原已删除的账户\n/purge <账户> - 彻底清空该账户的回收站（需确认）\n/confirm <确认码> - 确认刚才的删除或清空\n/cancel - 取消\n/whoami - 查看你的身份\n/help - 显示本帮助".into(),
            "Cloud Home 管理員機器人\n/pending - 查看待處理的註冊申請\n/users [pending|approved|rejected] - 列出帳戶\n/status - 硬碟與帳戶概況\n/delete <帳戶> - 刪除帳戶（需確認）\n/restore <帳戶> - 還原已刪除的帳戶\n/purge <帳戶> - 徹底清空該帳戶的資源回收筒（需確認）\n/confirm <確認碼> - 確認剛才的刪除或清空\n/cancel - 取消\n/whoami - 查看你的身份\n/help - 顯示本說明".into(),
        )
    }

    pub fn usage(self, cmd: &str) -> String {
        self.pick(
            format!("Usage: /{cmd} <email or phone> [email|phone]"),
            format!("用法：/{cmd} <邮箱或手机号> [email|phone]"),
            format!("用法：/{cmd} <信箱或手機號> [email|phone]"),
        )
    }

    pub fn users_usage(self) -> String {
        self.pick(
            "Usage: /users [pending|approved|rejected]".into(),
            "用法：/users [pending|approved|rejected]".into(),
            "用法：/users [pending|approved|rejected]".into(),
        )
    }

    pub fn users_header(self, shown: usize, total: usize) -> String {
        self.pick(
            format!("{total} account(s) (showing {shown}):"),
            format!("共 {total} 个账户（显示 {shown} 个）："),
            format!("共 {total} 個帳戶（顯示 {shown} 個）："),
        )
    }

    pub fn no_users(self) -> String {
        self.pick(
            "No accounts match.".into(),
            "没有符合的账户。".into(),
            "沒有符合的帳戶。".into(),
        )
    }

    pub fn status_summary(
        self,
        version: &str,
        drive_ok: bool,
        active: usize,
        pending: usize,
        rejected: usize,
        deleted: usize,
    ) -> String {
        let drive = match (drive_ok, self.0) {
            (true, ChatLanguage::En) => "connected",
            (false, ChatLanguage::En) => "NOT available",
            (true, ChatLanguage::ZhHans) => "已连接",
            (false, ChatLanguage::ZhHans) => "不可用",
            (true, ChatLanguage::ZhHant) => "已連接",
            (false, ChatLanguage::ZhHant) => "不可用",
        };
        self.pick(
            format!("Cloud Home v{version}\nDrive: {drive}\nAccounts: {active} active, {pending} pending, {rejected} rejected, {deleted} deleted"),
            format!("Cloud Home v{version}\n硬盘：{drive}\n账户：{active} 个可用，{pending} 个待批准，{rejected} 个已拒绝，{deleted} 个已删除"),
            format!("Cloud Home v{version}\n硬碟：{drive}\n帳戶：{active} 個可用，{pending} 個待批准，{rejected} 個已拒絕，{deleted} 個已刪除"),
        )
    }

    pub fn account_not_found(self) -> String {
        self.pick(
            "No such account.".into(),
            "找不到这个账户。".into(),
            "找不到這個帳戶。".into(),
        )
    }

    pub fn account_ambiguous(self) -> String {
        self.pick(
            "That identifier matches more than one account. Add \"email\" or \"phone\" after it.".into(),
            "该标识命中多个账户。请在后面加上 email 或 phone。".into(),
            "該標識命中多個帳戶。請在後面加上 email 或 phone。".into(),
        )
    }

    pub fn confirm_delete(self, name: &str, ident: &str, code: &str, minutes: u64) -> String {
        self.pick(
            format!("Delete account \"{name}\" ({ident})?\nIts files move to the trash and the account is disabled; /restore brings it back.\nTo proceed send /confirm {code} within {minutes} minutes, or /cancel."),
            format!("删除账户「{name}」（{ident}）？\n其文件会进入回收站，账户被停用；之后可用 /restore 还原。\n请在 {minutes} 分钟内发送 /confirm {code} 继续，或发送 /cancel 取消。"),
            format!("刪除帳戶「{name}」（{ident}）？\n其檔案會進入資源回收筒，帳戶被停用；之後可用 /restore 還原。\n請在 {minutes} 分鐘內傳送 /confirm {code} 繼續，或傳送 /cancel 取消。"),
        )
    }

    pub fn confirm_purge(self, name: &str, ident: &str, code: &str, minutes: u64) -> String {
        self.pick(
            format!("PURGE the trash of \"{name}\" ({ident})?\nThis permanently deletes every file in that account's trash from the drive. It is IRREVERSIBLE and cannot be undone.\nTo proceed send /confirm {code} within {minutes} minutes, or /cancel."),
            format!("彻底清空「{name}」（{ident}）的回收站？\n这会把该账户回收站里的所有文件从硬盘上永久删除，无法恢复。\n请在 {minutes} 分钟内发送 /confirm {code} 继续，或发送 /cancel 取消。"),
            format!("徹底清空「{name}」（{ident}）的資源回收筒？\n這會把該帳戶資源回收筒裡的所有檔案從硬碟上永久刪除，無法復原。\n請在 {minutes} 分鐘內傳送 /confirm {code} 繼續，或傳送 /cancel 取消。"),
        )
    }

    pub fn nothing_to_confirm(self) -> String {
        self.pick(
            "Nothing is waiting for confirmation.".into(),
            "没有等待确认的操作。".into(),
            "沒有等待確認的操作。".into(),
        )
    }

    pub fn confirmation_expired(self) -> String {
        self.pick(
            "That confirmation expired. Nothing was changed.".into(),
            "确认已过期，没有做任何改动。".into(),
            "確認已過期，沒有做任何變更。".into(),
        )
    }

    pub fn confirmation_wrong(self) -> String {
        self.pick(
            "Wrong code. The pending operation was cancelled and nothing was changed.".into(),
            "确认码不对。待确认的操作已取消，没有做任何改动。".into(),
            "確認碼不對。待確認的操作已取消，沒有做任何變更。".into(),
        )
    }

    pub fn cancelled(self) -> String {
        self.pick(
            "Cancelled.".into(),
            "已取消。".into(),
            "已取消。".into(),
        )
    }

    pub fn account_deleted(self, name: &str, items: u64) -> String {
        self.pick(
            format!("Deleted \"{name}\" ({items} item(s) moved to the trash)."),
            format!("已删除「{name}」（{items} 个项目进入回收站）。"),
            format!("已刪除「{name}」（{items} 個項目進入資源回收筒）。"),
        )
    }

    pub fn account_restored(self, name: &str, items: u64) -> String {
        self.pick(
            format!("Restored \"{name}\" ({items} item(s) brought back)."),
            format!("已还原「{name}」（还原 {items} 个项目）。"),
            format!("已還原「{name}」（還原 {items} 個項目）。"),
        )
    }

    pub fn trash_purged(self, name: &str, items: u64, bytes: u64) -> String {
        let mb = bytes as f64 / 1_048_576.0;
        self.pick(
            format!("Purged the trash of \"{name}\": {items} item(s), {mb:.1} MB freed."),
            format!("已清空「{name}」的回收站：{items} 个项目，释放 {mb:.1} MB。"),
            format!("已清空「{name}」的資源回收筒：{items} 個項目，釋放 {mb:.1} MB。"),
        )
    }

    pub fn drive_unavailable(self) -> String {
        self.pick(
            "The drive is not available. Nothing was changed.".into(),
            "硬盘不可用，没有做任何改动。".into(),
            "硬碟不可用，沒有做任何變更。".into(),
        )
    }

    pub fn not_done(self) -> String {
        self.pick(
            "Not done: the account is not in a state where this applies (already deleted, not deleted, or not approved).".into(),
            "未执行：该账户当前状态不适用此操作（已删除、未删除或未通过审批）。".into(),
            "未執行：該帳戶目前狀態不適用此操作（已刪除、未刪除或未通過審批）。".into(),
        )
    }

    pub fn whoami(self, label: &str, provider: &str, id: &str) -> String {
        self.pick(
            format!("You are admin \"{label}\" ({provider}:{id})."),
            format!("你是管理员「{label}」（{provider}:{id}）。"),
            format!("你是管理員「{label}」（{provider}:{id}）。"),
        )
    }

    // ---- 告警（R-065） ----

    pub fn install_button(self) -> String {
        self.pick("Install now".into(), "立即安装".into(), "立即安裝".into())
    }

    pub fn update_available(self, version: &str) -> String {
        self.pick(
            format!("Update available: Cloud Home {version}. Tap the button to install it; the service restarts afterwards."),
            format!("有新版本：Cloud Home {version}。点击按钮即可安装，安装后服务会重启。"),
            format!("有新版本：Cloud Home {version}。點擊按鈕即可安裝，安裝後服務會重啟。"),
        )
    }

    pub fn update_started(self, version: &str) -> String {
        self.pick(
            format!("Installing {version}. The service will restart in a moment."),
            format!("正在安装 {version}，服务稍后会重启。"),
            format!("正在安裝 {version}，服務稍後會重啟。"),
        )
    }

    pub fn update_gone(self) -> String {
        self.pick(
            "That update is no longer the one on offer. Nothing was installed.".into(),
            "该更新已不是当前可装的版本，没有安装任何东西。".into(),
            "該更新已不是目前可裝的版本，沒有安裝任何東西。".into(),
        )
    }

    pub fn update_busy(self, transfers: usize) -> String {
        self.pick(
            format!("Not installed: {transfers} upload/download(s) are in progress. Try again later."),
            format!("未安装：有 {transfers} 个上传/下载正在进行，请稍后再试。"),
            format!("未安裝：有 {transfers} 個上傳/下載正在進行，請稍後再試。"),
        )
    }

    pub fn update_refused(self) -> String {
        self.pick(
            "Not installed: online updates are disabled or an update is already running.".into(),
            "未安装：在线更新未启用，或已有更新正在进行。".into(),
            "未安裝：線上更新未啟用，或已有更新正在進行。".into(),
        )
    }

    pub fn drive_unplugged(self) -> String {
        self.pick(
            "The data drive was unplugged. Writes are refused until it is back.".into(),
            "数据盘已被拔出，接回之前写入操作会被拒绝。".into(),
            "資料碟已被拔出，接回之前寫入操作會被拒絕。".into(),
        )
    }

    pub fn drive_plugged(self) -> String {
        self.pick(
            "The data drive is back.".into(),
            "数据盘已重新接上。".into(),
            "資料碟已重新接上。".into(),
        )
    }

    pub fn low_disk(self, free_percent: u64, free_gb: u64) -> String {
        self.pick(
            format!("Low disk space: {free_percent}% free ({free_gb} GB left) on the data drive."),
            format!("磁盘空间不足：数据盘仅剩 {free_percent}%（约 {free_gb} GB）。"),
            format!("磁碟空間不足：資料碟僅剩 {free_percent}%（約 {free_gb} GB）。"),
        )
    }

    pub fn admin_auth_failures(self, count: usize, minutes: u64, ip: &str) -> String {
        self.pick(
            format!("Security: {count} failed admin-endpoint sign-ins in {minutes} minutes (latest from {ip})."),
            format!("安全提醒：{minutes} 分钟内管理端点验证失败 {count} 次（最近一次来自 {ip}）。"),
            format!("安全提醒：{minutes} 分鐘內管理端點驗證失敗 {count} 次（最近一次來自 {ip}）。"),
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
