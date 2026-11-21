use crate::auth::Claims;
use jsonwebtoken::{EncodingKey, Header, encode};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub(crate) fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

pub(crate) fn make_claims(
    role: &str,
    app_metadata: serde_json::Value,
    exp_offset_secs: i64,
) -> Claims {
    Claims {
        sub: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        aud: "authenticated".to_string(),
        role: role.to_string(),
        email: Some("fabricated@example.com".to_string()),
        app_metadata,
        exp: now_secs() + exp_offset_secs,
        iat: now_secs() - 10,
    }
}

pub(crate) fn sign(claims: &Claims, secret: &str) -> String {
    let decode_key =
        EncodingKey::from_secret(secret.as_bytes()).expect("cấu hình JWT secret không hợp lệ");
    encode(&Header::default(), claims, &decode_key).unwrap()
}

pub(crate) fn service_role_token(secret: &str) -> String {
    let claims = make_claims("service_role", serde_json::json!({}), 3600);
    sign(&claims, secret)
}
pub(crate) async fn signup_and_login(email: &str, password: &str) -> String {
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

    resp["access_token"].as_str().unwrap().to_string()
}

pub(crate) fn unique_test_email(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}-{nanos}@example.com")
}
