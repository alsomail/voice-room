-- T-10001: 默认 super_admin 种子数据
-- 参见 doc/protocol.md §六 6.3 "初始数据" 说明
--
-- password_hash 是 bcrypt(cost=12) 散列，对应明文密码:
--   admin_password_change_me
--
-- ⚠️  IMPORTANT: 首次部署后必须立即通过管理后台修改此密码！
--     生产环境禁止使用本文件中的默认密码。

INSERT INTO admins (username, password_hash, role, display_name, is_active)
VALUES (
    'super_admin',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5uWei',
    'super_admin',
    'Super Administrator',
    TRUE
)
ON CONFLICT (username) DO NOTHING;
