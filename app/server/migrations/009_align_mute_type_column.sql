-- T-00029 / R1 P0-2: 列名全链路对齐 — `type` → `mute_type`
--
-- 背景：008 初版迁移使用了 SQL 关键字 `type` 作为列名，而 Rust 写入 / Admin 查询
-- 全部使用 `mute_type` 字段名，导致任何 INSERT/SELECT 都会以
-- `column "mute_type" does not exist` 失败（参见审查报告 P0-2）。
--
-- 本迁移在保持幂等的前提下，将 `room_mute_records` 表的 `type` 列重命名为
-- `mute_type`，并同步重建对应 CHECK 约束与索引（兼容老版 008 已部署环境）。
-- 对于全新部署环境，008 已直接创建 `mute_type` 列，本脚本所有分支均为 no-op。

DO $$
BEGIN
    -- 仅当 (a) `type` 列仍存在且 (b) `mute_type` 列尚不存在 时执行重命名。
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name   = 'room_mute_records'
          AND column_name  = 'type'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name   = 'room_mute_records'
          AND column_name  = 'mute_type'
    ) THEN
        ALTER TABLE room_mute_records RENAME COLUMN "type" TO mute_type;
    END IF;
END $$;

-- 同步 CHECK 约束（旧约束名由 PG 自动生成或匿名，统一显式命名 chk_mute_type）。
-- 使用 IF NOT EXISTS 思路：先 DROP 同名约束（若存在），再 ADD。
ALTER TABLE room_mute_records
    DROP CONSTRAINT IF EXISTS chk_mute_type;
ALTER TABLE room_mute_records
    ADD CONSTRAINT chk_mute_type CHECK (mute_type IN ('mic','chat'));

-- 旧版索引若引用 `type` 列已随 RENAME COLUMN 自动迁移，无需重建。
-- 但为兼容历史索引名残留，统一保证目标索引存在：
CREATE INDEX IF NOT EXISTS idx_mute_records_target_type_ts
    ON room_mute_records(target_user_id, mute_type, created_at DESC);
