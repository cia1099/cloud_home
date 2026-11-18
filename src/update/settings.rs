//! 更新设置：模式 / 时间窗 / 通道。`.env` 给默认值，`update/settings.json` 可在运行时覆盖。

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// 只检查并提示，由人点击安装。
    Manual,
    /// 在时间窗内、设备空闲时自动安装。
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    /// 仅正式版（GitHub 的 `latest`，不含预发布）。
    Stable,
    /// 含预发布（rc）版本，供自己的设备先行验证。
    Beta,
}

impl std::str::FromStr for Mode {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "manual" => Ok(Self::Manual),
            "auto" => Ok(Self::Auto),
            other => bail!("UPDATE_MODE 只能是 manual 或 auto，收到 {other:?}"),
        }
    }
}

impl std::str::FromStr for Channel {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "stable" => Ok(Self::Stable),
            "beta" => Ok(Self::Beta),
            other => bail!("UPDATE_CHANNEL 只能是 stable 或 beta，收到 {other:?}"),
        }
    }
}

/// 一天内的时间窗（分钟，本地时间），允许跨午夜（如 `23:00-02:00`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    start: u32,
    end: u32,
}

impl Window {
    /// 解析 `HH:MM-HH:MM`。
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        let (a, b) = s
            .split_once('-')
            .with_context(|| format!("时间窗格式应为 HH:MM-HH:MM，收到 {s:?}"))?;
        let start = parse_hm(a)?;
        let end = parse_hm(b)?;
        if start == end {
            bail!("时间窗起止不能相同：{s:?}");
        }
        Ok(Self { start, end })
    }

    /// `minute` 为当日第几分钟（0..1440）。起点含、终点不含。
    pub fn contains(&self, minute: u32) -> bool {
        if self.start < self.end {
            minute >= self.start && minute < self.end
        } else {
            minute >= self.start || minute < self.end
        }
    }
}

fn parse_hm(s: &str) -> anyhow::Result<u32> {
    let (h, m) = s
        .trim()
        .split_once(':')
        .with_context(|| format!("时间应为 HH:MM，收到 {s:?}"))?;
    let h: u32 = h.parse().with_context(|| format!("小时无效：{s:?}"))?;
    let m: u32 = m.parse().with_context(|| format!("分钟无效：{s:?}"))?;
    if h > 23 || m > 59 {
        bail!("时间超出范围：{s:?}");
    }
    Ok(h * 60 + m)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub mode: Mode,
    /// `HH:MM-HH:MM`，本地时间。
    pub window: String,
    pub channel: Channel,
    pub check_interval_hours: u64,
}

impl Settings {
    pub fn window(&self) -> anyhow::Result<Window> {
        Window::parse(&self.window)
    }

    /// 校验（保存前调用，避免把坏值写盘后每次启动都失败）。
    pub fn validate(&self) -> anyhow::Result<()> {
        self.window()?;
        if self.check_interval_hours == 0 {
            bail!("check_interval_hours 必须 >= 1");
        }
        Ok(())
    }

    fn file(root: &Path) -> PathBuf {
        root.join("update").join("settings.json")
    }

    /// 以 `defaults`（来自 .env）为底，叠加磁盘上的覆盖；文件缺失/损坏则用默认。
    pub fn load(root: &Path, defaults: Settings) -> Settings {
        let Ok(bytes) = std::fs::read(Self::file(root)) else {
            return defaults;
        };
        match serde_json::from_slice::<Settings>(&bytes) {
            Ok(s) if s.validate().is_ok() => s,
            _ => {
                tracing::warn!("update/settings.json 无法解析，改用 .env 默认值");
                defaults
            }
        }
    }

    pub fn save(&self, root: &Path) -> anyhow::Result<()> {
        self.validate()?;
        let path = Self::file(root);
        std::fs::create_dir_all(path.parent().expect("has parent"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_simple_and_wrapping() {
        let w = Window::parse("03:00-05:00").unwrap();
        assert!(w.contains(3 * 60));
        assert!(w.contains(4 * 60 + 59));
        assert!(!w.contains(5 * 60));
        assert!(!w.contains(2 * 60 + 59));

        let night = Window::parse("23:00-02:00").unwrap();
        assert!(night.contains(23 * 60 + 30));
        assert!(night.contains(60));
        assert!(!night.contains(12 * 60));
    }

    #[test]
    fn window_rejects_garbage() {
        for bad in [
            "",
            "3-5",
            "25:00-01:00",
            "03:60-04:00",
            "03:00-03:00",
            "03:00",
        ] {
            assert!(Window::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn mode_and_channel_parse() {
        assert_eq!("AUTO".parse::<Mode>().unwrap(), Mode::Auto);
        assert_eq!("beta".parse::<Channel>().unwrap(), Channel::Beta);
        assert!("sometimes".parse::<Mode>().is_err());
    }
}
