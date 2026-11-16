use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use uuid::Uuid;

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
/// Giải mã payload của JWT MÀ KHÔNG kiểm tra chữ ký.
/// CHỈ dùng để xem thử cấu trúc dữ liệu — KHÔNG được dùng cho logic xác thực thật.
/// Bước 4.2 sẽ thay thế hoàn toàn bằng verify có chữ ký.
fn decode_unverified(token: &str) -> Result<Claims, String> {
    let payload_b64 = token
        .split('.')
        .nth(1)
        .ok_or_else(|| "JWT không đúng định dạng (thiếu phần payload)".to_string())?;
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| format!("giải mã base64 thất bại: {e}"))?;

    serde_json::from_slice(&payload_bytes).map_err(|e| format!("parse JSON thất bại: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkNGY0NzAwYy1jZDdiLTQ1ODYtODVhNC1lZjgzMjY1ODRjZDEiLCJhdWQiOiJhdXRoZW50aWNhdGVkIiwiZXhwIjoxNzk0ODA2Njk4LCJpYXQiOjE3OTQ4MDYzOTgsImVtYWlsIjoidHVuZ0BleGFtcGxlLmNvbSIsInBob25lIjoiIiwiYXBwX21ldGFkYXRhIjp7InByb3ZpZGVyIjoiZW1haWwiLCJwcm92aWRlcnMiOlsiZW1haWwiXX0sInVzZXJfbWV0YWRhdGEiOnsiZW1haWwiOiJ0dW5nQGV4YW1wbGUuY29tIiwiZW1haWxfdmVyaWZpZWQiOnRydWUsInBob25lX3ZlcmlmaWVkIjpmYWxzZSwic3ViIjoiZDRmNDcwMGMtY2Q3Yi00NTg2LTg1YTQtZWY4MzI2NTg0Y2QxIn0sInJvbGUiOiIiLCJhYWwiOiJhYWwxIiwiYW1yIjpbeyJtZXRob2QiOiJwYXNzd29yZCIsInRpbWVzdGFtcCI6MTc5NDgwNjM5OH1dLCJzZXNzaW9uX2lkIjoiYTY3OGQ1YmUtNDBkMi00ODc2LWE5NGItZjg3MTcyNGI1ZmFiIiwiaXNfYW5vbnltb3VzIjpmYWxzZX0.G0T7lnB_j9JZgqMgU4aF8Ewpa6sBvYB2FZtbFNZTYdM";
    const DEMO_EMAIL: &str = "tung@example.com";

    #[test]
    fn decode_unverified_maps_fields_correctly() {
        let claims = decode_unverified(DEMO_TOKEN).unwrap();

        assert_eq!(claims.aud, "authenticated");
        assert_eq!(claims.role, "");
        assert_eq!(claims.email, Some(DEMO_EMAIL.to_string()));
    }
}
