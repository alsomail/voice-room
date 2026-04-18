-- ============================================================
-- 权限隔离脚本：在所有业务表创建完成后执行
-- 执行方式：psql -h localhost -U postgres -d voiceroom -f scripts/dev/grant-permissions.sql
-- 幂等：可重复执行，GRANT/REVOKE 均为幂等操作
-- ============================================================

-- app_server_user: C 端业务表权限
GRANT USAGE ON SCHEMA public TO app_server_user;
GRANT SELECT, INSERT, UPDATE ON TABLE users TO app_server_user;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO app_server_user;

-- app_server_user: 禁止访问管理表
REVOKE ALL ON TABLE admins FROM app_server_user;
REVOKE ALL ON TABLE admin_logs FROM app_server_user;

-- admin_server_user: 全权
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO admin_server_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO admin_server_user;
