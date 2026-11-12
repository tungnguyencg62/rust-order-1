#!/bin/bash
set -euo pipefail

# Nạp toàn bộ biến trong .env vào shell hiện tại
set -a
source .env
set +a

echo "== 1. Trạng thái các container =="
docker compose ps

echo
echo "== 2. Đăng ký tài khoản GoTrue (email ngẫu nhiên, tránh trùng) =="
EMAIL="smoketest-$(date +%s)@example.com"
PASSWORD="SmokeTest123!"

SIGNUP=$(curl -s -X POST http://127.0.0.1:9999/signup \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}")
echo "$SIGNUP" | jq .

echo
echo "== 3. Đăng nhập, lấy access_token =="
LOGIN=$(curl -s -X POST "http://127.0.0.1:9999/token?grant_type=password" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}")
ACCESS_TOKEN=$(echo "$LOGIN" | jq -r .access_token)
echo "Nhận được access_token, độ dài: ${#ACCESS_TOKEN} ký tự"

echo
echo "== 4. Giải mã claims trong JWT =="
PAYLOAD=$(echo "$ACCESS_TOKEN" | cut -d. -f2 | tr '_-' '/+')
case $((${#PAYLOAD} % 4)) in
  2) PAYLOAD="${PAYLOAD}==" ;;
  3) PAYLOAD="${PAYLOAD}=" ;;
esac
echo "$PAYLOAD" | base64 -d | jq .

echo
echo "== 5. Gọi HTTP API của Centrifugo: publish =="
PUBLISH=$(curl -sS -X POST http://127.0.0.1:8000/api/publish \
  -H "X-API-Key: ${CENTRIFUGO_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"channel":"smoke-test","data":{"msg":"xin chao"}}')
echo "$PUBLISH" | jq .

echo
echo "== 6. Gọi HTTP API của Centrifugo: channels (xem có ai đang subscribe) =="
CHANNELS=$(curl -sS -X POST http://127.0.0.1:8000/api/channels \
  -H "X-API-Key: ${CENTRIFUGO_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{}')
echo "$CHANNELS" | jq .

echo
echo "== Hoàn tất. Tài khoản test: $EMAIL =="