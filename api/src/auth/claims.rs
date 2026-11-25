use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, errors::ErrorKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

/// Phải khớp chính xác GOTRUE_JWT_EXP trong docker-compose.yml.
/// Đây là khoảng hở tối đa denylist cần che — không hơn, không kém.
pub const MAX_TOKEN_LIFETIME_SECS: i64 = 300;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Claims {
    pub sub: Uuid,
    pub session_id: Uuid,
    pub aud: String,
    pub role: String,
    pub email: Option<String>,
    #[serde(default)]
    pub app_metadata: serde_json::Value,
    pub exp: i64,
    pub iat: i64,
}

impl Claims {
    pub fn custom_role(&self) -> Option<&str> {
        self.app_metadata.get("role").and_then(|v| v.as_str())
    }

    pub fn remaining_ttl_secs(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        (self.exp - now).max(1)
    }
}

pub fn verify(token: &str, secret: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["authenticated"]);

    let key = DecodingKey::from_secret(secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("cấu hình JWT secret không hợp lệ: {e}")))?;

    let data = decode::<Claims>(token, &key, &validation)
        .map_err(|e| AppError::Unauthorized(describe_error(&e)))?;

    Ok(data.claims)
}

fn describe_error(e: &jsonwebtoken::errors::Error) -> String {
    match e.kind() {
        ErrorKind::ExpiredSignature => "token đã hết hạn".to_string(),
        ErrorKind::InvalidSignature => "chữ ký token không hợp lệ".to_string(),
        ErrorKind::InvalidAudience => "token không đúng đối tượng sử dụng".to_string(),
        _ => "token không hợp lệ".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{signup_and_login, unique_test_email};

    fn test_secret() -> String {
        std::env::var("JWT_SECRET").expect("thiếu JWT_SECRET — đã `source api/.env` chưa?")
    }

    #[tokio::test]
    async fn verify_accepts_valid_token_and_decodes_claims() {
        let email = unique_test_email("test-claims");
        let token = signup_and_login(&email, "TestPassword123!").await;

        let claims = verify(&token, &test_secret()).unwrap();

        assert_eq!(claims.aud, "authenticated");
        assert_eq!(claims.email.as_deref(), Some(email.as_str()));
    }

    #[tokio::test]
    async fn verify_rejects_token_signed_with_wrong_secret() {
        let email = unique_test_email("test-claims");
        let token = signup_and_login(&email, "TestPassword123!").await;

        let result = verify(&token, "secret-hoan-toan-sai");

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn verify_rejects_garbage_token() {
        let result = verify("khong.phai.jwt", &test_secret()).unwrap_err();
        let _ = result; // chỉ cần xác nhận là Err, không cần kiểm tra nội dung
    }

    #[test]
    fn custom_role_reads_from_app_metadata() {
        let claims = crate::test_support::make_claims(
            "authenticated",
            serde_json::json!({"role": "merchant"}),
            300,
        );
        assert_eq!(claims.custom_role(), Some("merchant"));
    }

    #[test]
    fn custom_role_is_none_when_absent() {
        let claims = crate::test_support::make_claims("authenticated", serde_json::json!({}), 300);
        assert_eq!(claims.custom_role(), None);
    }
}
