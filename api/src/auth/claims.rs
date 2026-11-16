use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, errors::ErrorKind};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub session_id: Uuid,
    pub aud: String,
    pub role: String,
    pub email: Option<String>,
    pub exp: i64,
    pub iat: i64,
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

    async fn signup_and_login() -> (String, String) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let email = format!("test-claims-{nanos}@example.com");
        let password = "TestPassword123!";

        let client = reqwest::Client::new();
        let body = serde_json::json!({ "email": email, "password": password });

        client
            .post("http://127.0.0.1:9999/signup")
            .json(&body)
            .send()
            .await
            .expect("gọi /signup thất bại — đảm bảo `docker compose up -d` đang chạy")
            .error_for_status()
            .expect("GoTrue trả lỗi khi signup");

        let resp: serde_json::Value = client
            .post("http://127.0.0.1:9999/token?grant_type=password")
            .json(&body)
            .send()
            .await
            .expect("gọi /token thất bại")
            .json()
            .await
            .expect("parse response /token thất bại");

        let token = resp["access_token"].as_str().unwrap().to_string();
        (token, email)
    }

    fn test_secret() -> String {
        std::env::var("JWT_SECRET").expect("thiếu JWT_SECRET — đã `source api/.env` chưa?")
    }

    #[tokio::test]
    async fn verify_accepts_valid_token_and_decodes_claims() {
        let (token, email) = signup_and_login().await;

        let claims = verify(&token, &test_secret()).unwrap();

        assert_eq!(claims.aud, "authenticated");
        assert_eq!(claims.email.as_deref(), Some(email.as_str()));
    }

    #[tokio::test]
    async fn verify_rejects_token_signed_with_wrong_secret() {
        let (token, _) = signup_and_login().await;

        let result = verify(&token, "secret-hoan-toan-sai");

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn verify_rejects_garbage_token() {
        let result = verify("khong.phai.jwt", &test_secret());
        assert!(result.is_err());
    }
}
