package com.voice.room.android.common

import com.voice.room.android.BuildConfig
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AppContainerTest {
    @Test
    fun `fromBuildConfig wires debug placeholders and config`() {
        val container = AppContainer.fromBuildConfig()

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
    }
}
