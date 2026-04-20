-- T-10001: admins 表初始化
-- 参见 doc/protocol.md §六 6.3
-- TDD 验收用例：
--   [x] username 有唯一约束（UNIQUE 关键字）
--   [x] password_hash 存储 bcrypt 散列（VARCHAR(200) 可容纳 60+ 字符的 bcrypt 输出）
--   [x] role 字段有 CHECK 约束（super_admin, operator, cs, finance）
--   [x] is_active 支持账号禁用
--   [x] last_login_at 可记录最近登录时间

CREATE TABLE IF NOT EXISTS admins (
    id            UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    username      VARCHAR(50)  NOT NULL UNIQUE,
    password_hash VARCHAR(200) NOT NULL,
    role          VARCHAR(20)  NOT NULL DEFAULT 'operator',
    display_name  VARCHAR(100),
    is_active     BOOLEAN      NOT NULL DEFAULT TRUE,
    last_login_at TIMESTAMPTZ,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- role 枚举约束：只允许四个合法值
ALTER TABLE admins ADD CONSTRAINT chk_admin_role
    CHECK (role IN ('super_admin', 'operator', 'cs', 'finance'));
