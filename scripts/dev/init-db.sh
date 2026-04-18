#!/bin/bash
set -e

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
  -- App Server 受限账号（只能操作 C 端业务表）
  DO \$\$ BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'app_server_user') THEN
      CREATE ROLE app_server_user WITH LOGIN PASSWORD 'app_server_pass';
    END IF;
  END \$\$;

  -- Admin Server 全权账号
  DO \$\$ BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'admin_server_user') THEN
      CREATE ROLE admin_server_user WITH LOGIN PASSWORD 'admin_server_pass';
    END IF;
  END \$\$;

  -- 基础连接权限
  GRANT CONNECT ON DATABASE voiceroom TO app_server_user;
  GRANT CONNECT ON DATABASE voiceroom TO admin_server_user;

  -- admin_server_user 拥有 public schema 全权（含后续新建表）
  GRANT ALL PRIVILEGES ON SCHEMA public TO admin_server_user;
  ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO admin_server_user;
  ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO admin_server_user;
EOSQL
