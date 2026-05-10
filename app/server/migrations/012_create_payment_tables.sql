-- T-00050: 订单与 SKU Schema 与迁移
-- 参见 doc/tds/server/T-00050.md
-- 协议参考: doc/protocol/payment_api.md §9.2
--
-- 幂等保证：全部使用 IF NOT EXISTS / ON CONFLICT DO NOTHING
-- 执行顺序: ENUM → payment_skus → payment_orders → rtdn_processed → 索引 → 种子

-- ─────────────────────────────────────────────────────────────
-- 1. ENUM 类型
-- ─────────────────────────────────────────────────────────────

DO $$ BEGIN
    CREATE TYPE payment_provider AS ENUM (
        'google_play',
        'apple_iap',
        'mock'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE payment_order_state AS ENUM (
        'PENDING',
        'VERIFYING',
        'VERIFIED',
        'CREDITED',
        'ACKED',
        'CANCELLED',
        'FAILED',
        'REFUNDED',
        'PENDING_GOOGLE'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- ─────────────────────────────────────────────────────────────
-- 2. payment_skus 表（参见 payment_api.md §9.2.1）
-- ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS payment_skus (
    sku_id              VARCHAR(64)     PRIMARY KEY,
    provider            payment_provider NOT NULL DEFAULT 'google_play',
    diamonds            BIGINT          NOT NULL CHECK (diamonds > 0),
    display_price_usd   NUMERIC(10, 2)  NOT NULL,
    display_price_local NUMERIC(12, 2),
    display_currency    VARCHAR(3),
    is_active           BOOLEAN         NOT NULL DEFAULT TRUE,
    sort_order          INT             NOT NULL DEFAULT 0,
    tag                 VARCHAR(32),
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_skus_active_sort
    ON payment_skus (is_active, sort_order);

-- ─────────────────────────────────────────────────────────────
-- 3. payment_orders 表（参见 payment_api.md §9.2.2）
-- ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS payment_orders (
    order_id            UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID            NOT NULL REFERENCES users(id),
    sku_id              VARCHAR(64)     NOT NULL REFERENCES payment_skus(sku_id),
    provider            payment_provider NOT NULL,
    purchase_token      TEXT,
    provider_order_id   VARCHAR(64),
    amount_micros       BIGINT,
    currency            VARCHAR(3),
    country_code        VARCHAR(2),
    state               payment_order_state NOT NULL DEFAULT 'PENDING',
    state_history       JSONB           NOT NULL DEFAULT '[]',
    risk_flags          TEXT[]          NOT NULL DEFAULT '{}',
    idempotency_key     VARCHAR(64),
    dev_mock_outcome    VARCHAR(16),
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT now(),
    verified_at         TIMESTAMPTZ,
    credited_at         TIMESTAMPTZ,
    acked_at            TIMESTAMPTZ,
    failed_at           TIMESTAMPTZ,
    failed_reason       VARCHAR(64)
);

-- 索引：用户订单按时间倒序（参见 §9.2.2）
CREATE INDEX IF NOT EXISTS idx_orders_user_created
    ON payment_orders (user_id, created_at DESC);

-- 唯一索引：同一 provider 下 purchase_token 不重复（空值除外）
CREATE UNIQUE INDEX IF NOT EXISTS uq_orders_provider_purchase_token
    ON payment_orders (provider, purchase_token)
    WHERE purchase_token IS NOT NULL;

-- 局部索引：非终态订单快速扫描（cron 对账使用）
CREATE INDEX IF NOT EXISTS idx_orders_state_pending
    ON payment_orders (state, created_at)
    WHERE state IN ('PENDING', 'VERIFYING', 'VERIFIED', 'CREDITED');

-- ─────────────────────────────────────────────────────────────
-- 4. rtdn_processed 幂等去重表（参见 payment_api.md §9.2.5）
-- ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS rtdn_processed (
    message_id          VARCHAR(64)     PRIMARY KEY,
    event_time_millis   BIGINT          NOT NULL,
    notification_kind   VARCHAR(32)     NOT NULL,
    purchase_token      TEXT,
    processed_at        TIMESTAMPTZ     NOT NULL DEFAULT now(),
    outcome             VARCHAR(32)     NOT NULL
);

-- ─────────────────────────────────────────────────────────────
-- 5. wallet_transactions 扩展：新增 source 字段
--    参见 payment_api.md §9.2.4
-- ─────────────────────────────────────────────────────────────
ALTER TABLE wallet_transactions
    ADD COLUMN IF NOT EXISTS source VARCHAR(64);

-- ─────────────────────────────────────────────────────────────
-- 6. 种子数据：5 档钻石 SKU（参见 payment_api.md §9.2.1 种子表）
--    INSERT ... ON CONFLICT DO NOTHING 保证幂等
-- ─────────────────────────────────────────────────────────────
INSERT INTO payment_skus
    (sku_id, provider, diamonds, display_price_usd, is_active, sort_order, tag)
VALUES
    ('diamond_60',   'google_play', 60,   0.99, TRUE, 10, NULL),
    ('diamond_300',  'google_play', 300,  4.99, TRUE, 20, NULL),
    ('diamond_600',  'google_play', 600,  9.99, TRUE, 30, 'hot'),
    ('diamond_1980', 'google_play', 1980, 29.99, TRUE, 40, 'best_value'),
    ('diamond_6480', 'google_play', 6480, 99.99, TRUE, 50, NULL)
ON CONFLICT (sku_id) DO NOTHING;
