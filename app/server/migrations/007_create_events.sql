-- T-00022: 创建 events 分区表 + 首日分区 + 索引
-- 幂等：使用 IF NOT EXISTS 和 DO block 确保重复执行安全

CREATE TABLE IF NOT EXISTS events
(
    id           UUID        NOT NULL DEFAULT gen_random_uuid(),
    user_id      UUID,
    device_id    VARCHAR(64) NOT NULL,
    event_name   VARCHAR(64) NOT NULL,
    properties   JSONB       NOT NULL DEFAULT '{}'::jsonb,
    session_id   VARCHAR(64),
    client_ts    TIMESTAMPTZ,
    server_ts    TIMESTAMPTZ NOT NULL DEFAULT now(),
    app_version  VARCHAR(16),
    os_version   VARCHAR(32),
    locale       VARCHAR(16),
    network_type VARCHAR(16),
    PRIMARY KEY (id, server_ts) -- 分区键需含 server_ts
) PARTITION BY RANGE (server_ts);

-- 使用 PL/pgSQL 动态创建当日分区（按 Asia/Riyadh 时区计算，UTC+3）
DO
$$
    DECLARE
        today          date        := (now() AT TIME ZONE 'Asia/Riyadh')::date;
        partition_name text;
        from_ts        timestamptz;
        to_ts          timestamptz;
    BEGIN
        partition_name := 'events_' || to_char(today, 'YYYYMMDD');
        from_ts        := (today::timestamp AT TIME ZONE 'Asia/Riyadh');
        to_ts          := ((today + 1)::timestamp AT TIME ZONE 'Asia/Riyadh');

        IF NOT EXISTS (SELECT 1
                       FROM pg_class c
                                JOIN pg_namespace n ON n.oid = c.relnamespace
                       WHERE c.relname = partition_name
                         AND n.nspname = 'public') THEN
            EXECUTE format(
                    'CREATE TABLE %I PARTITION OF events FOR VALUES FROM (%L) TO (%L)',
                    partition_name, from_ts, to_ts
                    );
        END IF;
    END
$$;

-- 索引（IF NOT EXISTS 幂等）
CREATE INDEX IF NOT EXISTS idx_events_user_ts
    ON events (user_id, server_ts DESC)
    WHERE user_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_events_name_ts
    ON events (event_name, server_ts DESC);
