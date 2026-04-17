package com.voice.room.android.common

import com.voice.room.android.BuildConfig
import com.voice.room.android.core.config.AppEnvironment
import com.voice.room.android.core.config.IRemoteConfigService
import com.voice.room.android.core.config.InMemoryRemoteConfigService
import com.voice.room.android.core.im.IIMService
import com.voice.room.android.core.im.NoOpIMService
import com.voice.room.android.core.media.IMediaService
import com.voice.room.android.core.media.NoOpMediaService
import com.voice.room.android.core.telemetry.IAnalyticsService
import com.voice.room.android.core.telemetry.ICrashReporter
import com.voice.room.android.core.telemetry.NoOpAnalyticsService
import com.voice.room.android.core.telemetry.NoOpCrashReporter
import com.voice.room.android.data.auth.DebugAuthService
import com.voice.room.android.data.gift.DebugGiftRepository
import com.voice.room.android.data.room.DebugRoomGateway
import com.voice.room.android.data.room.DebugRoomSyncService
import com.voice.room.android.data.wallet.DebugWalletRepository
import com.voice.room.android.domain.auth.IAuthService
import com.voice.room.android.domain.gift.IGiftRepository
import com.voice.room.android.domain.room.IRoomGateway
import com.voice.room.android.domain.room.IRoomSyncService
import com.voice.room.android.domain.wallet.IWalletRepository

data class AppContainer(
    val environment: AppEnvironment,
    val analyticsService: IAnalyticsService,
    val crashReporter: ICrashReporter,
    val remoteConfigService: IRemoteConfigService,
    val mediaService: IMediaService,
    val imService: IIMService,
    val authService: IAuthService,
    val roomGateway: IRoomGateway,
    val roomSyncService: IRoomSyncService,
    val walletRepository: IWalletRepository,
    val giftRepository: IGiftRepository
) {
    companion object {
        fun fromBuildConfig(): AppContainer {
            val environment = AppEnvironment.fromBuildConfig(
                environmentName = BuildConfig.APP_ENVIRONMENT,
                apiBaseUrl = BuildConfig.API_BASE_URL,
                wsUrl = BuildConfig.WS_URL,
                analyticsEndpoint = BuildConfig.ANALYTICS_ENDPOINT
            )

            return AppContainer(
                environment = environment,
                analyticsService = NoOpAnalyticsService(),
                crashReporter = NoOpCrashReporter(),
                remoteConfigService = InMemoryRemoteConfigService(),
                mediaService = NoOpMediaService(),
                imService = NoOpIMService(),
                authService = DebugAuthService(),
                roomGateway = DebugRoomGateway(),
                roomSyncService = DebugRoomSyncService(),
                walletRepository = DebugWalletRepository(),
                giftRepository = DebugGiftRepository()
            )
        }
    }
}
