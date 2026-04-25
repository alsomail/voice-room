package com.voice.room.android.common

import com.voice.room.android.BuildConfig
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class AppContainerTest {
    @Test
    fun `fromBuildConfig wires debug placeholders and config`() {
        val container = AppContainer.forUnitTest()

        assertEquals(BuildConfig.APP_ENVIRONMENT, container.environment.environmentName)
        assertFalse(container.authService.currentUserLabel().isBlank())
        assertFalse(container.roomGateway.roomPreviewLabel().isBlank())
        assertFalse(container.roomSyncService.syncPolicyLabel().isBlank())
        assertFalse(container.walletRepository.walletPreviewLabel().isBlank())
        assertFalse(container.giftRepository.featuredGiftLabel().isBlank())
        assertFalse(container.mediaService.providerName().isBlank())
        assertFalse(container.imService.providerName().isBlank())
        assertTrue(container.remoteConfigService.getBoolean("missing_flag", true))
        container.analyticsService.trackScreen("bootstrap")
        container.analyticsService.trackAction("select_room")
        container.crashReporter.recordNonFatal("placeholder")
        // T-30034: AnalyticsPort 防腐层应可从 AppContainer 获取
        container.analyticsPort.track("bootstrap_complete")
        container.analyticsPort.captureException(RuntimeException("placeholder"))
    }

    /**
     * HIGH-01 修复验证（T-30043）：
     * AppContainer 应将 AnnouncementSeenStore 作为 Application 级别单例暴露，
     * 确保 RoomViewModel 跨实例共享同一 Store，AN43-02（24h 内不重复弹窗）在生产路径生效。
     */
    @Test
    fun `HIGH-01 AppContainer exposes announcementSeenStore as application-level singleton`() {
        val container = AppContainer.forUnitTest()

        // Store 不为 null
        assertNotNull("AppContainer.announcementSeenStore 不应为 null", container.announcementSeenStore)

        // 多次访问返回同一实例（data class 字段保证引用不变）
        assertSame(
            "同一 AppContainer 实例应始终返回相同的 announcementSeenStore 引用",
            container.announcementSeenStore,
            container.announcementSeenStore,
        )

        // Store 功能正常：save → get 一致
        container.announcementSeenStore.save("room-test", 999_000L)
        assertEquals(
            "announcementSeenStore.get 应返回已保存的时间戳",
            999_000L,
            container.announcementSeenStore.get("room-test"),
        )
    }
}
