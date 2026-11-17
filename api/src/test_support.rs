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
