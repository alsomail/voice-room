-- Migration 011: 贵族体系 E-09 (T-00065)
-- 创建 noble_tiers, user_nobles, noble_history, noble_global_broadcast_log 表
-- 并插入 6 档种子数据

-- ─── noble_tiers ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS noble_tiers (
    tier_id              VARCHAR(32)     PRIMARY KEY,
    name_en              VARCHAR(64)     NOT NULL,
    name_ar              VARCHAR(64)     NOT NULL,
    level                SMALLINT        NOT NULL UNIQUE CHECK (level BETWEEN 1 AND 6),
    monthly_diamonds     BIGINT          NOT NULL CHECK (monthly_diamonds > 0),
    monthly_usd          NUMERIC(10,2)   NOT NULL,
    usd_sku_id           VARCHAR(64)     NULL,
    privileges           JSONB           NOT NULL,
    icon_url             TEXT            NOT NULL,
    frame_url            TEXT            NOT NULL,
    entrance_animation_url TEXT          NULL,
    bgm_url              TEXT            NULL,
    badge_color          VARCHAR(16)     NOT NULL,
    bubble_style_id      VARCHAR(32)     NOT NULL,
    is_active            BOOLEAN         NOT NULL DEFAULT TRUE,
    created_at           TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

-- ─── user_nobles ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS user_nobles (
    user_id                 UUID            PRIMARY KEY REFERENCES users(id),
    tier_id                 VARCHAR(32)     NOT NULL REFERENCES noble_tiers(tier_id),
    start_at                TIMESTAMPTZ     NOT NULL,
    current_period_start    TIMESTAMPTZ     NOT NULL,
    expire_at               TIMESTAMPTZ     NOT NULL,
    auto_renew              BOOLEAN         NOT NULL DEFAULT TRUE,
    renew_channel           VARCHAR(16)     NOT NULL,
    failed_renew_count      INT             NOT NULL DEFAULT 0,
    total_paid_diamonds     BIGINT          NOT NULL DEFAULT 0,
    total_paid_usd_micros   BIGINT          NOT NULL DEFAULT 0,
    last_changed_msg_id     VARCHAR(64)     NULL,
    created_at              TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_nobles_expire
    ON user_nobles (expire_at);

CREATE INDEX IF NOT EXISTS idx_user_nobles_auto_renew
    ON user_nobles (auto_renew, expire_at)
    WHERE auto_renew = TRUE;

-- ─── noble_history ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS noble_history (
    id          BIGSERIAL       PRIMARY KEY,
    user_id     UUID            NOT NULL,
    event       VARCHAR(32)     NOT NULL,
    from_tier   VARCHAR(32)     NULL,
    to_tier     VARCHAR(32)     NULL,
    payload     JSONB           NULL,
    actor       VARCHAR(64)     NOT NULL DEFAULT 'system',
    created_at  TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_noble_history_user_id
    ON noble_history (user_id, created_at DESC);

-- ─── noble_global_broadcast_log ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS noble_global_broadcast_log (
    id              BIGSERIAL   PRIMARY KEY,
    user_id         UUID        NOT NULL,
    kind            VARCHAR(32) NOT NULL,
    broadcast_date  DATE        NOT NULL DEFAULT CURRENT_DATE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, kind, broadcast_date)
);

-- ─── privileges JSONB 校验函数 ────────────────────────────────────────────────
CREATE OR REPLACE FUNCTION noble_privileges_validate(p JSONB)
RETURNS BOOLEAN LANGUAGE plpgsql AS $$
BEGIN
    IF p IS NULL THEN
        RAISE EXCEPTION 'privileges must not be null';
    END IF;
    IF NOT (p ? 'mic_priority') THEN
        RAISE EXCEPTION 'privileges must contain mic_priority';
    END IF;
    IF NOT (p ? 'gift_discount') THEN
        RAISE EXCEPTION 'privileges must contain gift_discount';
    END IF;
    IF NOT (p ? 'monthly_stipend') THEN
        RAISE EXCEPTION 'privileges must contain monthly_stipend';
    END IF;
    RETURN TRUE;
END;
$$;

-- ─── 6 档种子数据（§10.2.4） ──────────────────────────────────────────────────

INSERT INTO noble_tiers (
    tier_id, name_en, name_ar, level,
    monthly_diamonds, monthly_usd, usd_sku_id,
    privileges,
    icon_url, frame_url, entrance_animation_url, bgm_url,
    badge_color, bubble_style_id
) VALUES
-- LV1 Knight
(
    'knight', 'Knight', 'فارس', 1,
    3000, 9.99, NULL,
    '{"badge":{"color":"#6B7280","shape":"shield","animated":false},
      "entry_effect":{"duration_ms":0,"scope":"marquee","marquee_color":"gray","user_can_disable":true},
      "chat_bubble":{"style_id":"knight","gradient":["#D1D5DB","#9CA3AF"],"border_color":"#6B7280","username_color":"#374151"},
      "audience_pin":{"scope":"none","rank_offset":0},
      "invisibility":{"scope":"none","always_visible_to":["admin"]},
      "bypass_password":{"enabled":false,"respect_room_owner_switch":true},
      "mic_priority":{"weight":1.0},
      "gift_discount":{"percent":0},
      "global_broadcast":{"enabled":false,"daily_limit":0},
      "vip_support":{"sla_minutes":60},
      "monthly_stipend":{"diamonds":0,"pay_immediately":false},
      "expiry":{"warn_days_before":3,"grace_days":7,"history_days":30}}'::jsonb,
    'https://cdn.voiceroom.app/nobles/knight_icon.svg',
    'https://cdn.voiceroom.app/nobles/knight_frame.png',
    NULL,
    NULL,
    '#6B7280',
    'knight'
),
-- LV2 Baron
(
    'baron', 'Baron', 'بارون', 2,
    10000, 29.99, NULL,
    '{"badge":{"color":"#059669","shape":"crown_small","animated":false},
      "entry_effect":{"duration_ms":3000,"scope":"marquee","marquee_color":"green","user_can_disable":true},
      "chat_bubble":{"style_id":"baron","gradient":["#A7F3D0","#34D399"],"border_color":"#059669","username_color":"#065F46"},
      "audience_pin":{"scope":"own_room","rank_offset":0},
      "invisibility":{"scope":"none","always_visible_to":["admin"]},
      "bypass_password":{"enabled":false,"respect_room_owner_switch":true},
      "mic_priority":{"weight":1.0},
      "gift_discount":{"percent":2},
      "global_broadcast":{"enabled":false,"daily_limit":0},
      "vip_support":{"sla_minutes":30},
      "monthly_stipend":{"diamonds":0,"pay_immediately":false},
      "expiry":{"warn_days_before":3,"grace_days":7,"history_days":30}}'::jsonb,
    'https://cdn.voiceroom.app/nobles/baron_icon.svg',
    'https://cdn.voiceroom.app/nobles/baron_frame.png',
    NULL,
    'https://cdn.voiceroom.app/nobles/baron_bgm.mp3',
    '#059669',
    'baron'
),
-- LV3 Viscount
(
    'viscount', 'Viscount', 'نبيل', 3,
    30000, 99.99, 'noble_viscount_30d',
    '{"badge":{"color":"#2563EB","shape":"crown_medium","animated":false},
      "entry_effect":{"duration_ms":5000,"scope":"half","marquee_color":"blue","user_can_disable":true},
      "chat_bubble":{"style_id":"viscount","gradient":["#BFDBFE","#60A5FA"],"border_color":"#2563EB","username_color":"#1E40AF"},
      "audience_pin":{"scope":"own_room","rank_offset":0},
      "invisibility":{"scope":"none","always_visible_to":["admin"]},
      "bypass_password":{"enabled":false,"respect_room_owner_switch":true},
      "mic_priority":{"weight":1.0},
      "gift_discount":{"percent":5},
      "global_broadcast":{"enabled":false,"daily_limit":0},
      "vip_support":{"sla_minutes":15},
      "monthly_stipend":{"diamonds":0,"pay_immediately":false},
      "expiry":{"warn_days_before":3,"grace_days":7,"history_days":30}}'::jsonb,
    'https://cdn.voiceroom.app/nobles/viscount_icon.svg',
    'https://cdn.voiceroom.app/nobles/viscount_frame.png',
    'https://cdn.voiceroom.app/nobles/viscount_entry.json',
    'https://cdn.voiceroom.app/nobles/viscount_bgm.mp3',
    '#2563EB',
    'viscount'
),
-- LV4 Earl
(
    'earl', 'Earl', 'أيرل', 4,
    100000, 299.99, 'noble_earl_30d',
    '{"badge":{"color":"#7C3AED","shape":"crown_large","animated":false},
      "entry_effect":{"duration_ms":6000,"scope":"half","marquee_color":"purple","user_can_disable":false},
      "chat_bubble":{"style_id":"earl","gradient":["#DDD6FE","#A78BFA"],"border_color":"#7C3AED","username_color":"#4C1D95"},
      "audience_pin":{"scope":"own_lobby","rank_offset":1},
      "invisibility":{"scope":"mic_only","always_visible_to":["admin"]},
      "bypass_password":{"enabled":false,"respect_room_owner_switch":true},
      "mic_priority":{"weight":1.5},
      "gift_discount":{"percent":8},
      "global_broadcast":{"enabled":false,"daily_limit":0},
      "vip_support":{"sla_minutes":10},
      "monthly_stipend":{"diamonds":0,"pay_immediately":false},
      "expiry":{"warn_days_before":3,"grace_days":7,"history_days":30}}'::jsonb,
    'https://cdn.voiceroom.app/nobles/earl_icon.svg',
    'https://cdn.voiceroom.app/nobles/earl_frame.png',
    'https://cdn.voiceroom.app/nobles/earl_entry.json',
    'https://cdn.voiceroom.app/nobles/earl_bgm.mp3',
    '#7C3AED',
    'earl'
),
-- LV5 Duke
(
    'duke', 'Duke', 'دوق', 5,
    300000, 999.99, 'noble_duke_30d',
    '{"badge":{"color":"#06B6D4","shape":"crown_large","animated":true},
      "entry_effect":{"duration_ms":8000,"scope":"fullscreen","marquee_color":"cyan","user_can_disable":false},
      "chat_bubble":{"style_id":"duke","gradient":["#CFFAFE","#22D3EE"],"border_color":"#06B6D4","username_color":"#0E7490"},
      "audience_pin":{"scope":"global","rank_offset":1},
      "invisibility":{"scope":"mic_and_audience","always_visible_to":["admin"]},
      "bypass_password":{"enabled":true,"respect_room_owner_switch":true},
      "mic_priority":{"weight":3.0},
      "gift_discount":{"percent":10},
      "global_broadcast":{"enabled":true,"daily_limit":1},
      "vip_support":{"sla_minutes":5},
      "monthly_stipend":{"diamonds":60000,"pay_immediately":true},
      "expiry":{"warn_days_before":3,"grace_days":7,"history_days":30}}'::jsonb,
    'https://cdn.voiceroom.app/nobles/duke_icon.svg',
    'https://cdn.voiceroom.app/nobles/duke_frame.png',
    'https://cdn.voiceroom.app/nobles/duke_entry.json',
    'https://cdn.voiceroom.app/nobles/duke_bgm.mp3',
    '#06B6D4',
    'duke'
),
-- LV6 King
(
    'king', 'King', 'ملك', 6,
    1000000, 3999.99, 'noble_king_30d',
    '{"badge":{"color":"#DC2626","shape":"crown_large","animated":true},
      "entry_effect":{"duration_ms":10000,"scope":"fullscreen","marquee_color":"red_gold","user_can_disable":false},
      "chat_bubble":{"style_id":"king","gradient":["#FCA5A5","#F59E0B"],"border_color":"#F59E0B","username_color":"#991B1B"},
      "audience_pin":{"scope":"global","rank_offset":1},
      "invisibility":{"scope":"all","always_visible_to":["admin"]},
      "bypass_password":{"enabled":true,"respect_room_owner_switch":true},
      "mic_priority":{"weight":10.0},
      "gift_discount":{"percent":15},
      "global_broadcast":{"enabled":true,"daily_limit":1},
      "vip_support":{"sla_minutes":5},
      "monthly_stipend":{"diamonds":200000,"pay_immediately":true},
      "expiry":{"warn_days_before":3,"grace_days":7,"history_days":30}}'::jsonb,
    'https://cdn.voiceroom.app/nobles/king_icon.svg',
    'https://cdn.voiceroom.app/nobles/king_frame.png',
    'https://cdn.voiceroom.app/nobles/king_entry.json',
    'https://cdn.voiceroom.app/nobles/king_bgm.mp3',
    '#DC2626',
    'king'
)
ON CONFLICT (tier_id) DO NOTHING;
