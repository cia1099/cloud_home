//! 第三方登录（OAuth / OIDC）的与厂商无关的接口层。
//!
//! 这里只定义「一家身份提供方要会什么」（[`OAuthProvider`]）和「服务端启用了哪些」
//! （[`OAuthRegistry`]）。Google / Apple 的真实客户端尚未接入，所以生产环境的
//! 注册表目前是空的；测试用 `FakeProvider`（仅 `cfg(test)`）走完整条登录链路，不碰网络。

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::{AppError, AppResult};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// 授权码换回的第三方资料。
#[derive(Debug, Clone)]
pub struct ExternalProfile {
    /// 第三方给出的稳定、不透明用户标识（OIDC 的 `sub`），存进 `user_identities.identifier`。
    pub subject: String,
    pub email: Option<String>,
    /// 第三方是否声明该邮箱已验证。未验证的邮箱绝不用于账户关联。
    pub email_verified: bool,
    /// 第三方给出的显示名。
    pub name: Option<String>,
}

/// 一家第三方身份提供方。
///
/// `redirect_uri` 的白名单校验、`state` / PKCE 的细节由具体实现负责；
/// 前端自己生成并核对 `state`，服务端只负责把它原样带进授权地址。
pub trait OAuthProvider: Send + Sync {
    /// provider 名，同时是 `user_identities.provider` 与 `managed_by` 的取值（`google` / `apple`）。
    fn name(&self) -> &'static str;

    /// 把用户送去授权的地址。
    fn authorize_url(&self, redirect_uri: &str, state: &str) -> AppResult<String>;

    /// 用授权码换取并校验第三方资料。码无效应返回 `AppError::Unauthorized`。
    fn exchange<'a>(
        &'a self,
        code: &'a str,
        redirect_uri: &'a str,
    ) -> BoxFuture<'a, AppResult<ExternalProfile>>;
}

/// 服务端启用的 provider 集合，外加总开关（`OAUTH_ENABLED`，默认关）。
#[derive(Clone)]
pub struct OAuthRegistry {
    enabled: bool,
    providers: BTreeMap<&'static str, Arc<dyn OAuthProvider>>,
}

impl OAuthRegistry {
    /// 目前没有真实 provider 可装配，所以只带开关；真实客户端接入时在此注册。
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            providers: BTreeMap::new(),
        }
    }

    #[cfg(test)]
    pub fn with(mut self, provider: impl OAuthProvider + 'static) -> Self {
        self.providers.insert(provider.name(), Arc::new(provider));
        self
    }

    /// 对外报告的已配置 provider：总开关关闭时一律为空，前端据此隐藏按钮。
    pub fn names(&self) -> Vec<&'static str> {
        if self.enabled {
            self.providers.keys().copied().collect()
        } else {
            Vec::new()
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 取某个 provider；开关关闭或未配置都是 404（功能不存在，而非客户端的错）。
    pub fn get(&self, name: &str) -> AppResult<Arc<dyn OAuthProvider>> {
        if !self.enabled {
            return Err(AppError::NotFound);
        }
        self.providers.get(name).cloned().ok_or(AppError::NotFound)
    }
}

/// 测试用的假 provider：预置「授权码 → 资料」，不联网。
#[cfg(test)]
pub struct FakeProvider {
    name: &'static str,
    codes: std::collections::HashMap<String, ExternalProfile>,
}

#[cfg(test)]
impl FakeProvider {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            codes: Default::default(),
        }
    }

    pub fn code(mut self, code: &str, profile: ExternalProfile) -> Self {
        self.codes.insert(code.to_string(), profile);
        self
    }
}

#[cfg(test)]
impl OAuthProvider for FakeProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    fn authorize_url(&self, redirect_uri: &str, state: &str) -> AppResult<String> {
        Ok(format!(
            "https://fake.example/{}/authorize?redirect_uri={redirect_uri}&state={state}",
            self.name
        ))
    }

    fn exchange<'a>(
        &'a self,
        code: &'a str,
        _redirect_uri: &'a str,
    ) -> BoxFuture<'a, AppResult<ExternalProfile>> {
        Box::pin(async move { self.codes.get(code).cloned().ok_or(AppError::Unauthorized) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> ExternalProfile {
        ExternalProfile {
            subject: "s1".into(),
            email: Some("a@b.com".into()),
            email_verified: true,
            name: None,
        }
    }

    #[tokio::test]
    async fn disabled_switch_hides_everything() {
        let reg = OAuthRegistry::new(false).with(FakeProvider::new("google"));
        assert!(!reg.enabled());
        assert!(reg.names().is_empty());
        assert!(matches!(reg.get("google"), Err(AppError::NotFound)));
    }

    #[tokio::test]
    async fn enabled_lists_and_resolves_configured_only() {
        let reg = OAuthRegistry::new(true).with(FakeProvider::new("google").code("c", profile()));
        assert_eq!(reg.names(), vec!["google"]);
        assert!(matches!(reg.get("apple"), Err(AppError::NotFound)));
        let p = reg.get("google").unwrap();
        assert_eq!(p.exchange("c", "x").await.unwrap().subject, "s1");
        assert!(matches!(
            p.exchange("bad", "x").await,
            Err(AppError::Unauthorized)
        ));
    }
}
