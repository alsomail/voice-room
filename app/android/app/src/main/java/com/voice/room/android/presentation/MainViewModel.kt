package com.voice.room.android.presentation

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import com.voice.room.android.common.AppContainer
import com.voice.room.android.core.config.AppEnvironment
import com.voice.room.android.core.telemetry.IAnalyticsService
import com.voice.room.android.domain.auth.IAuthService
import com.voice.room.android.domain.gift.IGiftRepository
import com.voice.room.android.domain.room.IRoomGateway
import com.voice.room.android.domain.room.IRoomSyncService
import com.voice.room.android.domain.wallet.IWalletRepository
import com.voice.room.android.feature.auth.AuthFeature
import com.voice.room.android.feature.profile.ProfileFeature
import com.voice.room.android.feature.room.RoomFeature

class MainViewModel(
    private val environment: AppEnvironment,
    private val analyticsService: IAnalyticsService,
    private val authService: IAuthService,
    private val roomGateway: IRoomGateway,
    private val roomSyncService: IRoomSyncService,
    private val walletRepository: IWalletRepository,
    private val giftRepository: IGiftRepository
) : ViewModel() {
    private var currentDestination = MainDestination.AUTH
    private var currentState = buildState(currentDestination)

    val uiState: MainUiState
        get() = currentState

    init {
        analyticsService.trackScreen("bootstrap_${currentDestination.name.lowercase()}")
    }

    fun onDestinationSelected(destination: MainDestination): MainUiState {
        currentDestination = destination
        analyticsService.trackAction("select_${destination.name.lowercase()}")
        currentState = buildState(destination)
        return currentState
    }

    private fun buildState(destination: MainDestination): MainUiState {
        val descriptor = when (destination) {
            MainDestination.AUTH -> AuthFeature.descriptor
            MainDestination.ROOM -> RoomFeature.descriptor
            MainDestination.PROFILE -> ProfileFeature.descriptor
        }

        return MainUiState(
            title = descriptor.title,
            description = descriptor.description,
            apiBaseUrl = environment.apiBaseUrl,
            wsUrl = environment.wsUrl,
            statusLines = listOf(
                authService.currentUserLabel(),
                roomGateway.roomPreviewLabel(),
                roomSyncService.syncPolicyLabel(),
                walletRepository.walletPreviewLabel(),
                giftRepository.featuredGiftLabel()
            ),
            warnings = environment.validateForPhysicalDevice()
        )
    }

    class Factory(
        private val container: AppContainer
    ) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T {
            return MainViewModel(
                environment = container.environment,
                analyticsService = container.analyticsService,
                authService = container.authService,
                roomGateway = container.roomGateway,
                roomSyncService = container.roomSyncService,
                walletRepository = container.walletRepository,
                giftRepository = container.giftRepository
            ) as T
        }
    }
}
