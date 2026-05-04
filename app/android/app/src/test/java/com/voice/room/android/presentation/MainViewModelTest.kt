package com.voice.room.android.presentation

import com.voice.room.android.core.config.AppEnvironment
import com.voice.room.android.core.telemetry.IAnalyticsService
import com.voice.room.android.domain.auth.IAuthService
import com.voice.room.android.domain.gift.IGiftRepository
import com.voice.room.android.domain.room.IRoomGateway
import com.voice.room.android.domain.room.IRoomSyncService
import com.voice.room.android.domain.wallet.IWalletRepository
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class MainViewModelTest {
    @Test
    fun `initial state shows auth bootstrap details`() {
        val analytics = RecordingAnalyticsService()
        val viewModel = MainViewModel(
            environment = AppEnvironment(
                environmentName = "dev",
                apiBaseUrl = "http://192.168.1.19:3000/api",
                wsUrl = "ws://192.168.1.19:3000/ws",
                analyticsEndpoint = "https://analytics-dev.example.com/collect"
            ),
            analyticsService = analytics,
            authService = object : IAuthService {
                override fun currentUserLabel(): String = "Guest bootstrap user"
            },
            roomGateway = object : IRoomGateway {
                override fun roomPreviewLabel(): String = "Room module reserved"
            },
            roomSyncService = object : IRoomSyncService {
                override fun syncPolicyLabel(): String = "Heartbeat and reconnect planned"
            },
            walletRepository = object : IWalletRepository {
                override fun walletPreviewLabel(): String = "Wallet module reserved"
                override suspend fun getBalance(): Result<Long> = Result.success(0L)
                override suspend fun listTxns(page: Int, size: Int) =
                    Result.success(com.voice.room.android.domain.wallet.TxnsPage(emptyList(), 0, page))
            },
            giftRepository = object : IGiftRepository {
                override fun featuredGiftLabel(): String = "Gift module reserved"
                override suspend fun listGifts(locale: String) = Result.success(emptyList<com.voice.room.android.domain.gift.GiftVO>())
            }
        )

        assertEquals("Auth Bootstrap", viewModel.uiState.title)
        assertTrue(viewModel.uiState.statusLines.contains("Guest bootstrap user"))
        assertEquals(listOf("screen:bootstrap_auth"), analytics.events)
    }

    @Test
    fun `selecting room updates the rendered state`() {
        val viewModel = MainViewModel(
            environment = AppEnvironment(
                environmentName = "dev",
                apiBaseUrl = "http://192.168.1.19:3000/api",
                wsUrl = "ws://192.168.1.19:3000/ws",
                analyticsEndpoint = "https://analytics-dev.example.com/collect"
            ),
            analyticsService = RecordingAnalyticsService(),
            authService = object : IAuthService {
                override fun currentUserLabel(): String = "Guest bootstrap user"
            },
            roomGateway = object : IRoomGateway {
                override fun roomPreviewLabel(): String = "Room module reserved"
            },
            roomSyncService = object : IRoomSyncService {
                override fun syncPolicyLabel(): String = "Heartbeat and reconnect planned"
            },
            walletRepository = object : IWalletRepository {
                override fun walletPreviewLabel(): String = "Wallet module reserved"
                override suspend fun getBalance(): Result<Long> = Result.success(0L)
                override suspend fun listTxns(page: Int, size: Int) =
                    Result.success(com.voice.room.android.domain.wallet.TxnsPage(emptyList(), 0, page))
            },
            giftRepository = object : IGiftRepository {
                override fun featuredGiftLabel(): String = "Gift module reserved"
                override suspend fun listGifts(locale: String) = Result.success(emptyList<com.voice.room.android.domain.gift.GiftVO>())
            }
        )

        val roomState = viewModel.onDestinationSelected(MainDestination.ROOM)

        assertEquals("Room Hall", roomState.title)
        assertTrue(roomState.statusLines.contains("Heartbeat and reconnect planned"))
        assertTrue(roomState.statusLines.contains("Gift module reserved"))
    }

    private class RecordingAnalyticsService : IAnalyticsService {
        val events = mutableListOf<String>()

        override fun trackScreen(screenName: String) {
            events += "screen:$screenName"
        }

        override fun trackAction(actionName: String) {
            events += "action:$actionName"
        }
    }
}
