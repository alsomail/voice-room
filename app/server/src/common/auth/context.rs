use uuid::Uuid;

/// JWT 校验成功后注入请求上下文的用户身份信息。
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
}
