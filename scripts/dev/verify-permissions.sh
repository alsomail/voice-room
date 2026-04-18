#!/usr/bin/env bash
# T-0000C 验收脚本：验证权限隔离矩阵
# 使用方法：./scripts/dev/verify-permissions.sh
# 前置条件：docker-compose up -d 且 PG 已 healthy

set -euo pipefail

CONTAINER=vr-postgres
DB=voiceroom

run_as_postgres() { docker exec "$CONTAINER" psql -U postgres -d $DB -c "$1" -q 2>&1; }
run_as_app()      { docker exec -e PGPASSWORD=app_server_pass   "$CONTAINER" psql -U app_server_user   -d $DB -c "$1" 2>&1; }
run_as_admin()    { docker exec -e PGPASSWORD=admin_server_pass "$CONTAINER" psql -U admin_server_user -d $DB -c "$1" 2>&1; }

PASS=0; FAIL=0
check() {
  local desc="$1" expect_ok="$2"; shift 2
  local result
  set +e
  result=$("$@" 2>&1)
  set -e
  if [ "$expect_ok" = "ok" ] && echo "$result" | grep -qiE "ERROR|FATAL|denied|permission"; then
    echo "❌ FAIL: $desc"; echo "   output: $result"; FAIL=$((FAIL+1))
  elif [ "$expect_ok" = "fail" ] && ! echo "$result" | grep -qiE "ERROR|FATAL|denied|permission"; then
    echo "❌ FAIL: $desc (expected permission error)"; echo "   output: $result"; FAIL=$((FAIL+1))
  else
    echo "✅ PASS: $desc"; PASS=$((PASS+1))
  fi
}

echo "=== T-0000C 权限验收 ==="

# 建测试表
run_as_postgres "DROP TABLE IF EXISTS admin_logs, admins, users CASCADE;"
run_as_postgres "CREATE TABLE users (id serial PRIMARY KEY, name text);"
run_as_postgres "CREATE TABLE admins (id serial PRIMARY KEY, name text);"
run_as_postgres "CREATE TABLE admin_logs (id serial PRIMARY KEY, msg text);"

# 将 SQL 文件复制进容器并执行
docker cp scripts/dev/grant-permissions.sql "$CONTAINER":/tmp/grant-permissions.sql
docker exec "$CONTAINER" psql -U postgres -d $DB -f /tmp/grant-permissions.sql -q

# 验收
check "app_server_user 可 INSERT users"        ok   run_as_app  "INSERT INTO users (name) VALUES ('test');"
check "app_server_user 可 SELECT users"        ok   run_as_app  "SELECT * FROM users LIMIT 1;"
check "app_server_user 可 UPDATE users"        ok   run_as_app  "UPDATE users SET name='x' WHERE id=1;"
check "app_server_user 无法 SELECT admins"     fail run_as_app  "SELECT * FROM admins LIMIT 1;"
check "app_server_user 无法 SELECT admin_logs" fail run_as_app  "SELECT * FROM admin_logs LIMIT 1;"
check "admin_server_user 可 INSERT admins"     ok   run_as_admin "INSERT INTO admins (name) VALUES ('admin');"
check "admin_server_user 可 SELECT users"      ok   run_as_admin "SELECT * FROM users LIMIT 1;"

# 幂等：再次执行不报错
docker exec "$CONTAINER" psql -U postgres -d $DB -f /tmp/grant-permissions.sql -q
echo "✅ PASS: 幂等执行（第二次无报错）"; PASS=$((PASS+1))

# 清理
run_as_postgres "DROP TABLE IF EXISTS admin_logs, admins, users CASCADE;"

echo ""
echo "=== 结果：$PASS 通过，$FAIL 失败 ==="
[ $FAIL -eq 0 ]

