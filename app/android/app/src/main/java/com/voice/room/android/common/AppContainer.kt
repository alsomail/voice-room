package com.voice.room.android.common

import com.voice.room.android.BuildConfig
import com.voice.room.android.core.analytics.AnalyticsPort
import com.voice.room.android.core.analytics.impl.NoopAnalytics
import com.voice.room.android.core.config.AppEnvironment
import com.voice.room.android.core.config.IRemoteConfigService
import com.voice.room.android.core.config.InMemoryRemoteConfigService
import com.voice.room.android.core.im.IIMService
import com.voice.room.android.core.im.NoOpIMService
import com.voice.room.android.core.media.IMediaService
import com.voice.room.android.core.media.NoOpMediaService
import com.voice.room.android.core.network.AppHttpClientFactory
import com.voice.room.android.core.network.AuthInterceptor
import com.voice.room.android.core.network.DefaultUnauthorizedHandler
import com.voice.room.android.core.network.NetworkClientConfig
import com.voice.room.android.core.telemetry.IAnalyticsService
import com.voice.room.android.core.telemetry.ICrashReporter
import com.voice.room.android.core.telemetry.NoOpAnalyticsService
import com.voice.room.android.core.telemetry.NoOpCrashReporter
import com.voice.room.android.core.ws.IWebSocketClient
import com.voice.room.android.core.ws.OkHttpWebSocketClient
import com.voice.room.android.data.auth.DebugAuthService
import com.voice.room.android.data.gift.DebugGiftRepository
import com.voice.room.android.data.remote.api.RoomApiService
import com.voice.room.android.data.remote.api.WalletApiService
import com.voice.room.android.data.wallet.RetrofitWalletRepository
import com.voice.room.android.data.ranking.RetrofitRankingRepository
import com.voice.room.android.data.remote.api.RankingApiService
import com.voice.room.android.domain.ranking.IRankingRepository
import com.voice.room.android.data.remote.api.UserApiService
import com.voice.room.android.data.room.DebugRoomGateway
import com.voice.room.android.data.room.DebugRoomSyncService
import com.voice.room.android.data.room.RetrofitRoomRepository
import com.voice.room.android.data.user.RetrofitUserRepository
import com.voice.room.android.domain.auth.IAuthService
import com.voice.room.android.domain.gift.IGiftRepository
import com.voice.room.android.domain.local.ITokenManager
import com.voice.room.android.domain.room.IRoomGateway
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.IRoomSyncService
import com.voice.room.android.domain.user.IUserRepository
import com.voice.room.android.domain.wallet.IWalletRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import retrofit2.Retrofit
import retrofit2.converter.gson.GsonConverterFactory

data class AppContainer(
    val environment: AppEnvironment,
    val analyticsPort: AnalyticsPort,
    val analyticsService: IAnalyticsService,
    val crashReporter: ICrashReporter,
    val remoteConfigService: IRemoteConfigService,
    val mediaService: IMediaService,
    val imService: IIMService,
    val authService: IAuthService,
    val roomGateway: IRoomGateway,
    val roomSyncService: IRoomSyncService,
    val walletRepository: IWalletRepository,
    val giftRepository: IGiftRepository,
    val rankingRepository: IRankingRepository,
    val roomRepository: IRoomRepository,
    val webSocketClient: IWebSocketClient,
    val tokenManager: ITokenManager,
    val userRepository: IUserRepository,
) {
    companion object {
        fun fromBuildConfig(): AppContainer {
            val environment = AppEnvironment.fromBuildConfig(
                environmentName = BuildConfig.APP_ENVIRONMENT,
                apiBaseUrl = BuildConfig.API_BASE_URL,
                wsUrl = BuildConfig.WS_URL,
                analyticsEndpoint = BuildConfig.ANALYTICS_ENDPOINT
            )

            // 鉴权组件：TokenManager（内存实现，debug 构建；生产应替换为 DataStore 版本）+ AuthInterceptor
            // HIGH-01 修复：POST /api/v1/rooms 需要 Authorization: Bearer <token>，必须注入 AuthInterceptor
            val tokenManager = object : ITokenManager {
                @Volatile private var token: String? = null
                override suspend fun saveToken(token: String) { this.token = token }
                override suspend fun getToken(): String? = token
                override suspend fun clearToken() { token = null }
            }
            val unauthorizedHandler = DefaultUnauthorizedHandler(tokenManager)
            val authInterceptor = AuthInterceptor(tokenManager, unauthorizedHandler)

            // Rooms API — 注入 AuthInterceptor 以支持 POST /rooms 鉴权
            val roomRetrofit = Retrofit.Builder()
                .baseUrl("${environment.apiBaseUrl}/v1/")
                .addConverterFactory(GsonConverterFactory.create())
                .client(AppHttpClientFactory.create(authInterceptor = authInterceptor))
                .build()
            val roomApiService = roomRetrofit.create(RoomApiService::class.java)

            // User API — 复用 roomRetrofit（已注入 AuthInterceptor，Bearer token 自动附加）
            val userApiService = roomRetrofit.create(UserApiService::class.java)
            val userRepository: IUserRepository = RetrofitUserRepository(userApiService)

            // Wallet API — 复用 roomRetrofit（已注入 AuthInterceptor）(T-30027)
            val walletApiService = roomRetrofit.create(WalletApiService::class.java)
            val walletRepository: IWalletRepository = RetrofitWalletRepository(walletApiService)

            // Gift API — 复用 roomRetrofit（已注入 AuthInterceptor）(T-30028)
            val giftApiService = roomRetrofit.create(
                com.voice.room.android.data.remote.api.GiftApiService::class.java
            )
            val giftRepository: IGiftRepository =
                com.voice.room.android.data.gift.RetrofitGiftRepository(giftApiService)

            // Ranking API — 复用 roomRetrofit（已注入 AuthInterceptor）(T-30033)
            val rankingApiService = roomRetrofit.create(RankingApiService::class.java)
            val rankingRepository: IRankingRepository = RetrofitRankingRepository(rankingApiService)

            // WebSocket 客户端 — 独立 IO 作用域，随 App 生命周期存在
            val wsHttpClient = AppHttpClientFactory.create(
                config = NetworkClientConfig(),
                authInterceptor = null
            )
            val wsScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
            val webSocketClient = OkHttpWebSocketClient(
                okHttpClient = wsHttpClient,
                scope = wsScope
            )

            return AppContainer(
                environment = environment,
                analyticsPort = NoopAnalytics(),
                analyticsService = NoOpAnalyticsService(),
                crashReporter = NoOpCrashReporter(),
                remoteConfigService = InMemoryRemoteConfigService(),
                mediaService = NoOpMediaService(),
                imService = NoOpIMService(),
                authService = DebugAuthService(),
                roomGateway = DebugRoomGateway(),
                roomSyncService = DebugRoomSyncService(),
                walletRepository = walletRepository,
                giftRepository = giftRepository,
                rankingRepository = rankingRepository,
                roomRepository = RetrofitRoomRepository(roomApiService),
                webSocketClient = webSocketClient,
                tokenManager = tokenManager,
                userRepository = userRepository,
            )
        }
    }
}
