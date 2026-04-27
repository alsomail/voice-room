#!/bin/bash
set -e

APP_SERVER_PASS="${APP_SERVER_PASS:?APP_SERVER_PASS env var is required}"
ADMIN_SERVER_PASS="${ADMIN_SERVER_PASS:?ADMIN_SERVER_PASS env var is required}"

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
  -- App Server 受限账号（只能操作 C 端业务表）
  DO \$\$ BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_server_user') THEN
      CREATE ROLE app_server_user WITH LOGIN PASSWORD '$APP_SERVER_PASS';
    END IF;
  END \$\$;

  -- Admin Server 全权账号
  DO \$\$ BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'admin_server_user') THEN
      CREATE ROLE admin_server_user WITH LOGIN PASSWORD '$ADMIN_SERVER_PASS';
    END IF;
  END \$\$;

  -- 基础连接权限
  GRANT CONNECT ON DATABASE voiceroom TO app_server_user;
  GRANT CONNECT ON DATABASE voiceroom TO admin_server_user;

  -- T-0000M：AppServer 需要在 public schema 下自建 _sqlx_app_migrations 登记表，
  -- 因此必须显式 GRANT CREATE ON SCHEMA public（幂等：DO 守卫不存在则授）。
  -- 撤掉了 scripts/dev/e2e-up.sh 的 inline 临时补丁。
  GRANT CREATE ON SCHEMA public TO app_server_user;

  -- admin_server_user 拥有 public schema 全权（含后续新建表）
  GRANT ALL PRIVILEGES ON SCHEMA public TO admin_server_user;
  ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO admin_server_user;
  ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO admin_server_user;
  -- T-0000N fix: 对迁移已建表补授（ALTER DEFAULT PRIVILEGES 仅对此命令执行后新建的表有效）
  GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO admin_server_user;
  GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO admin_server_user;
EOSQL
