-- T-00019: 礼物配置表 + 8 款 MVP 种子数据
-- Ref: doc/tds/server/T-00019.md

CREATE TABLE IF NOT EXISTS gifts (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code           VARCHAR(32) NOT NULL UNIQUE, -- 稳定标识，如 'rose_01'
    name_en        VARCHAR(64) NOT NULL,
    name_ar        VARCHAR(64) NOT NULL,
    icon_url       TEXT NOT NULL,
    price          BIGINT NOT NULL CHECK (price >= 1),
    tier           SMALLINT NOT NULL CHECK (tier BETWEEN 1 AND 5),
    effect_level   SMALLINT NOT NULL DEFAULT 1, -- 1:none 2:slot 3:bottom 4:fullscreen 5:fullscreen+border
    animation_url  TEXT,
    sort_order     INT NOT NULL DEFAULT 0,
    is_active      BOOLEAN NOT NULL DEFAULT true,
    is_deleted     BOOLEAN NOT NULL DEFAULT false,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_gifts_active_order ON gifts(tier, sort_order) WHERE is_active AND NOT is_deleted;

-- Seed 8 款 MVP 礼物（中东偏好，见 doc/product/phase1_gift_economy.md §3）
INSERT INTO gifts (code, name_en, name_ar, icon_url, price, tier, effect_level, sort_order) VALUES
 ('rose_01',     'Rose',           'وردة',              '/assets/gifts/rose.png',       1,    1, 1, 10),
 ('coffee_01',   'Arabic Coffee',  'قهوة عربية',        '/assets/gifts/coffee.png',    10,    2, 2, 20),
 ('kaaba_01',    'Kaaba Candle',   'شمعة الكعبة',       '/assets/gifts/kaaba.png',     10,    2, 2, 21),
 ('camel_01',    'Desert Camel',   'جمل',               '/assets/gifts/camel.png',     66,    3, 3, 30),
 ('falcon_01',   'Golden Falcon',  'صقر ذهبي',          '/assets/gifts/falcon.png',    88,    3, 3, 31),
 ('moon_786',    'Bismillah Moon', 'هلال بسم الله',     '/assets/gifts/moon786.png',  786,    4, 4, 40),
 ('castle_01',   'Royal Castle',   'قصر ملكي',          '/assets/gifts/castle.png',   520,    4, 4, 41),
 ('diamond_ring','Diamond Ring',   'خاتم الماس',        '/assets/gifts/diamond.png', 1314,    5, 5, 50)
ON CONFLICT (code) DO NOTHING;
