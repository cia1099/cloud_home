use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// 身份提供方。`password` 是凭证（存哈希），其余是可用于识别账户的标识。
/// 第三方 provider（`google` / `apple`）不在此列举：代码一律通过
/// `managed_by` 是否为空来判断归属，不关心具体是哪一家。
pub const PROVIDER_PASSWORD: &str = "password";
pub const PROVIDER_EMAIL: &str = "email";
pub const PROVIDER_PHONE: &str = "phone";

/// 用户可自行增删改的标识（与第三方托管的相对）。
pub const USER_OWNED_PROVIDERS: [&str; 2] = [PROVIDER_EMAIL, PROVIDER_PHONE];

/// 数据库中的用户行——仅剩个人资料，凭证与标识都在 `user_identities`。
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct User {
    pub id: String,
    pub username: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_active: i64,
    pub deleted_at: Option<String>,
}

/// 一条身份记录：凭证（password）或标识（email/phone/google/apple）。
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct UserIdentity {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub identifier: String,
    pub secret: Option<String>,
    pub verified_at: Option<String>,
    /// `None` 表示用户自有、可编辑；`Some("google")` 等表示第三方托管、本地只读。
    pub managed_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl UserIdentity {
    pub fn is_user_owned(&self) -> bool {
        self.managed_by.is_none()
    }
}

/// 注册请求体：email 与 phone 至少提供一个。
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserDto {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
}

/// 登录请求体：`identifier` 可为 email 或手机号。
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginDto {
    pub identifier: String,
    pub password: String,
}

/// 修改显示名。
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProfileDto {
    pub username: String,
}

/// 修改密码。
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordDto {
    pub current_password: String,
    pub new_password: String,
}

/// 请求「旧标识」验证码，用于随后的增改删。
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChallengeDto {
    /// `email` 或 `phone`。
    pub provider: String,
}

/// 新增或修改标识。
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangeIdentifierDto {
    pub provider: String,
    pub new_value: String,
    pub current_password: String,
    pub old_code: String,
}

/// 删除标识。
#[derive(Debug, Deserialize, ToSchema)]
pub struct RemoveIdentifierDto {
    pub provider: String,
    pub current_password: String,
    pub old_code: String,
}

/// 请求「验证本标识」的验证码。
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyRequestDto {
    pub provider: String,
}

/// 提交验证码以将标识标记为已验证。
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyIdentifierDto {
    pub provider: String,
    pub code: String,
}

/// 管理端定位账户：标识 + 可选的 provider 消歧。
#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminTargetDto {
    pub identifier: String,
    #[serde(default)]
    pub provider: Option<String>,
}

/// 对外返回的单条标识信息。
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IdentityResponse {
    pub provider: String,
    pub identifier: String,
    pub verified: bool,
    /// 非 null 表示由该第三方托管，本地不可编辑。
    pub managed_by: Option<String>,
}

impl From<&UserIdentity> for IdentityResponse {
    fn from(i: &UserIdentity) -> Self {
        Self {
            provider: i.provider.clone(),
            identifier: i.identifier.clone(),
            verified: i.verified_at.is_some(),
            managed_by: i.managed_by.clone(),
        }
    }
}

/// 对外返回的用户信息（不含任何凭证）。
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    /// 是否已设置密码；纯第三方登录账户为 false。
    pub has_password: bool,
    pub identities: Vec<IdentityResponse>,
    pub created_at: String,
}

impl UserResponse {
    pub fn build(user: &User, identities: &[UserIdentity]) -> Self {
        let pick = |p: &str| {
            identities
                .iter()
                .find(|i| i.provider == p)
                .map(|i| i.identifier.clone())
        };
        Self {
            id: user.id.clone(),
            username: user.username.clone(),
            email: pick(PROVIDER_EMAIL),
            phone: pick(PROVIDER_PHONE),
            has_password: identities.iter().any(|i| i.provider == PROVIDER_PASSWORD),
            identities: identities
                .iter()
                .filter(|i| i.provider != PROVIDER_PASSWORD)
                .map(IdentityResponse::from)
                .collect(),
            created_at: user.created_at.clone(),
        }
    }
}

/// 注册 / 登录成功后返回的凭证信息。
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub expires_at: String,
    pub user: UserResponse,
}

/// 管理端账户列表中的一项。
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUserItem {
    pub id: String,
    pub username: String,
    pub is_active: bool,
    pub deleted_at: Option<String>,
    pub identities: Vec<IdentityResponse>,
}

/// `GET /admin/users` 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUserListResponse {
    pub items: Vec<AdminUserItem>,
}

/// 管理端删除 / 还原的结果。
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminActionResponse {
    pub user_id: String,
    pub username: String,
    /// 删除时为移入回收站的顶层项目数；还原时为还原的项目数。
    pub affected_items: u64,
}
