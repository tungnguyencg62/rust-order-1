# Hạ tầng Phase 1

| Service    | Port (host) | Ghi chú                                   |
|------------|-------------|--------------------------------------------|
| Postgres   | 5432        | DB `postgres` (auth) + DB `app` (ứng dụng) |
| Adminer    | 8081        | http://127.0.0.1:8081                     |
| GoTrue     | 9999        | http://127.0.0.1:9999                     |
| Redis      | 6379        | cần `-a $REDIS_PASSWORD`                   |
| Centrifugo | 8000        | admin panel + HTTP API `/api/*`            |

Mật khẩu/secret: xem `.env` (không commit). Cấu trúc biến: xem `.env.example`.

Lệnh thường dùng:
- `docker compose up -d` / `down` (giữ dữ liệu) / `down -v` (xóa sạch)
- `./scripts/smoke-test.sh` — kiểm tra nhanh toàn bộ hạ tầng