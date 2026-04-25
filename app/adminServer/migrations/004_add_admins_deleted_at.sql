-- T-10001 修复: 为 admins 表补充 deleted_at 软删除列
--
-- 背景：app/adminServer/src/modules/auth/repository.rs::PgAdminRepository::find_by_username
--       的 SQL 语句使用了 `WHERE username = $1 AND deleted_at IS NULL` 子句，
--       但 001_create_admins.sql 未定义 deleted_at 列，导致生产环境调用必然报
--       PostgreSQL 错误 42703（column "admins.deleted_at" does not exist）。
--
-- 修复策略：与 users 表保持一致的软删除语义 —
--   1. 新增 deleted_at TIMESTAMPTZ 列（默认 NULL = 未删除）
--   2. 删除旧的全局 UNIQUE(username) 约束（CREATE TABLE 中的 UNIQUE 关键字）
--   3. 重建一个仅作用于 deleted_at IS NULL 行的条件唯一索引，
--      允许同一 username 在软删后被复用，与 idx_users_phone_active 风格对齐。
--
-- 幂等：使用 IF NOT EXISTS / IF EXISTS，可重复运行。

ALTER TABLE admins ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

-- 移除原 CREATE TABLE 中 username 上的 UNIQUE 约束（PostgreSQL 自动以
-- "<table>_<column>_key" 命名）。如果约束已经被人工改名/删除，IF EXISTS 兜底。
ALTER TABLE admins DROP CONSTRAINT IF EXISTS admins_username_key;

-- 仅对未软删除的管理员账号保证 username 唯一
CREATE UNIQUE INDEX IF NOT EXISTS idx_admins_username_active
    ON admins(username)
    WHERE deleted_at IS NULL;
