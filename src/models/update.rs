//! 在线更新管理端点的请求 / 响应体。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::update::Snapshot;
use crate::update::settings::{Channel, Mode, Settings};

/// `GET /admin/update` 等端点的响应：更新器当前状态 + 生效的设置。
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateStatusResponse {
    /// 是否由安装目录运行。`false`（开发机 `cargo run`）时检查 / 安装 / 改设置都会 409。
    pub enabled: bool,
    /// 编译目标三元组（与 `/health` 一致）。
    pub target: &'static str,
    #[serde(flatten)]
    pub snapshot: Snapshot,
    pub settings: Settings,
}

/// `POST /admin/update/install` 请求体。整个体可省略。
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallUpdateDto {
    /// 为 `true` 时即使有上传 / 下载在进行也照常安装（会中断它们）。默认 `false`。
    #[serde(default)]
    pub force: bool,
}

/// `PUT /admin/update/settings` 请求体：只改给出的字段，其余保持不变。
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateSettingsDto {
    pub mode: Option<Mode>,
    /// `HH:MM-HH:MM`，本地时间，可跨午夜。
    pub window: Option<String>,
    pub channel: Option<Channel>,
    /// 1..=720。
    pub check_interval_hours: Option<u64>,
}

impl UpdateSettingsDto {
    /// 叠加到当前设置上，得到待校验的新设置。
    pub fn apply(self, mut base: Settings) -> Settings {
        if let Some(v) = self.mode {
            base.mode = v;
        }
        if let Some(v) = self.window {
            base.window = v.trim().to_string();
        }
        if let Some(v) = self.channel {
            base.channel = v;
        }
        if let Some(v) = self.check_interval_hours {
            base.check_interval_hours = v;
        }
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_only_overrides_given_fields() {
        let base = Settings {
            mode: Mode::Manual,
            window: "03:00-05:00".into(),
            channel: Channel::Stable,
            check_interval_hours: 6,
        };
        let dto = UpdateSettingsDto {
            mode: Some(Mode::Auto),
            window: Some(" 01:00-02:00 ".into()),
            channel: None,
            check_interval_hours: None,
        };
        let s = dto.apply(base);
        assert_eq!(s.mode, Mode::Auto);
        assert_eq!(s.window, "01:00-02:00");
        assert_eq!(s.channel, Channel::Stable);
        assert_eq!(s.check_interval_hours, 6);
    }
}
