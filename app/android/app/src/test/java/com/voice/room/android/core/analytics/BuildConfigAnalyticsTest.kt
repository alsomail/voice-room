package com.voice.room.android.core.analytics

import com.voice.room.android.BuildConfig
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * TDD 单元测试 — BuildConfig.SENTRY_DSN 注入验证 (T-30034 A34-03)
 *
 * BC-01: BuildConfig.SENTRY_DSN 字段存在（编译时验证）
 * BC-02: BuildConfig.SENTRY_DSN 值不为 null
 * BC-03: BuildConfig.BUILD_TYPE 存在（dev/prod 区分基础）
 * BC-04: AppContainer 包含 analyticsPort 且不为 null
 */
class BuildConfigAnalyticsTest {

    // ─────────────────────────────────────────────
    // BC-01/02: BuildConfig.SENTRY_DSN 字段存在且不为 null
    // ─────────────────────────────────────────────

    @Test
    fun BC01_sentryDsn_fieldExists_andIsNotNull() {
        // 如果编译通过，说明 buildConfigField("SENTRY_DSN") 已成功注入
        val dsn: String = BuildConfig.SENTRY_DSN
        assertNotNull("BuildConfig.SENTRY_DSN 不应为 null", dsn)
    }

    // ─────────────────────────────────────────────
    // BC-03: BUILD_TYPE 存在（支持 dev/prod 环境区分）
    // ─────────────────────────────────────────────

    @Test
    fun BC03_buildType_exists() {
        val buildType: String = BuildConfig.BUILD_TYPE
        assertNotNull("BuildConfig.BUILD_TYPE 不应为 null", buildType)
        assertTrue("BUILD_TYPE 不应为空字符串", buildType.isNotEmpty())
    }

    // ─────────────────────────────────────────────
    // BC-04: AppContainer.analyticsPort 可用
    // ─────────────────────────────────────────────

    @Test
    fun BC04_appContainer_hasAnalyticsPort() {
        val container = com.voice.room.android.common.AppContainer.forUnitTest()
        assertNotNull("AppContainer 应包含 analyticsPort", container.analyticsPort)
    }
}
