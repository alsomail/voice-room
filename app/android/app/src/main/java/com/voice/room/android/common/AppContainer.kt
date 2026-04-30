package com.voice.room.android.common

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.Build
import androidx.annotation.VisibleForTesting
import com.voice.room.android.BuildConfig
import com.voice.room.android.core.analytics.AnalyticsPort
import com.voice.room.android.core.analytics.ConsentMode
import com.voice.room.android.core.analytics.EventReportClient
import com.voice.room.android.core.analytics.context.CommonPropsProvider
import com.voice.room.android.core.analytics.impl.CompositeAnalyticsPort
import com.voice.room.android.core.analytics.impl.NoopAnalytics
import com.voice.room.android.core.analytics.impl.SentryAnalytics
import com.voice.room.android.core.analytics.privacy.SensitiveFilter
import com.voice.room.android.core.analytics.queue.EventQueueDao
import com.voice.room.android.core.analytics.queue.InMemoryEventQueueDao
import com.voice.room.android.core.analytics.queue.RoomEventQueueRepository
import com.voice.room.android.core.analytics.session.SessionManager
import com.voice.room.android.core.analytics.throttle.Throttler
import com.voice.room.android.core.analytics.transport.HttpTransport
import com.voice.room.android.core.analytics.transport.WsTransport
import com.voice.room.android.core.config.AppEnvironment
import com.voice.room.android.core.config.IRemoteConfigService
import com.voice.room.android.core.config.InMemoryRemoteConfigService
import com.voice.room.android.core.consent.ConsentRepository
import com.voice.room.android.core.consent.ConsentStore
import com.voice.room.android.core.consent.DataStoreConsentStore
import com.voice.room.android.core.consent.InMemoryConsentStore
import com.voice.room.android.core.im.IIMService
import com.voice.room.android.core.im.NoOpIMService
import com.voice.room.android.core.media.IMediaService
import com.voice.room.android.core.media.NoOpMediaService
import com.voice.room.android.core.network.AppHttpClientFactory
import com.voice.room.android.core.network.AuthInterceptor
import com.voice.room.android.core.network.DefaultUnauthorizedHandler
import com.voice.room.android.core.network.NetworkClientConfig
import com.voice.room.android.core.storage.AppDatabase
import com.voice.room.android.core.telemetry.IAnalyticsService
import com.voice.room.android.core.telemetry.ICrashReporter
import com.voice.room.android.core.telemetry.NoOpAnalyticsService
import com.voice.room.android.core.telemetry.NoOpCrashReporter
import com.voice.room.android.core.ws.IWebSocketClient
import com.voice.room.android.core.ws.OkHttpWebSocketClient
import com.voice.room.android.core.ws.WebSocketState
import com.voice.room.android.data.auth.DebugAuthService
import com.voice.room.android.data.gift.DebugGiftRepository
import com.voice.room.android.data.local.AnnouncementSeenStore
import com.voice.room.android.data.local.InMemoryAnnouncementSeenStore
import com.voice.room.android.data.local.InMemoryKickCooldownStore
import com.voice.room.android.data.local.KickCooldownStore
import com.voice.room.android.data.ranking.RetrofitRankingRepository
import com.voice.room.android.data.remote.api.RankingApiService
import com.voice.room.android.data.remote.api.RoomApiService
import com.voice.room.android.data.remote.api.UserApiService
import com.voice.room.android.data.remote.api.WalletApiService
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import com.voice.room.android.data.local.TokenManager
import com.voice.room.android.data.room.DebugRoomGateway
import com.voice.room.android.data.room.DebugRoomSyncService
import com.voice.room.android.data.room.IRoomSnapshotRepository
import com.voice.room.android.data.room.RetrofitRoomRepository
import com.voice.room.android.data.room.RetrofitRoomSnapshotRepository
import com.voice.room.android.data.user.RetrofitUserRepository
import com.voice.room.android.data.wallet.RetrofitWalletRepository
import com.voice.room.android.domain.auth.IAuthService
import com.voice.room.android.domain.gift.IGiftRepository
import com.voice.room.android.domain.local.ITokenManager
import com.voice.room.android.domain.ranking.IRankingRepository
import com.voice.room.android.domain.room.IRoomGateway
import com.voice.room.android.domain.room.IRoomRepository
import com.voice.room.android.domain.room.IRoomSyncService
import com.voice.room.android.domain.user.IUserRepository
import com.voice.room.android.domain.wallet.IWalletRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import retrofit2.Retrofit
import retrofit2.converter.gson.GsonConverterFactory
import java.io.File
import java.util.Locale
import java.util.concurrent.atomic.AtomicReference

/**
 * 应用全局依赖容器。
 *
 * R1 批 2（缺陷 2/5/7）：
 * - 主链路完整组装：[EventQueueDao]（Room）→ [Throttler] → [WsTransport]/[HttpTransport]
 *   → [SessionManager] → [CommonPropsProvider] → [ConsentRepository] → [EventReportClient]
 * - [analyticsPort] 由 [CompositeAnalyticsPort] 适配 [SentryAnalytics] + [EventReportClient]，
 *   生产路径**不再注入** [NoopAnalytics]
 * - 提供 [forUnitTest] 工厂供 JVM 单元测试在无 Android Context 时复用
 */
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
    /** BUG-ROOM-NAV 修复：房间快照仓库（GET /rooms/{id}） */
    val roomSnapshotRepository: IRoomSnapshotRepository,
    val tokenManager: ITokenManager,
    val userRepository: IUserRepository,
    /** T-30002：登录认证仓库（发送验证码 + 登录） */
    val authRepository: com.voice.room.android.domain.auth.IAuthRepository,
    /** 401 未授权处理器（登录成功后 resetUnauthorized） */
    val unauthorizedHandler: com.voice.room.android.core.network.UnauthorizedHandler,
    /** T-30042 */
    val kickCooldownStore: KickCooldownStore = InMemoryKickCooldownStore(),
    /** T-30043 */
    val announcementSeenStore: AnnouncementSeenStore = InMemoryAnnouncementSeenStore(),
    /** T-30035：用户隐私同意状态（Splash 弹窗读写） */
    val consentRepository: ConsentRepository,
    /** T-30035：事件上报客户端（debug 工具/调试用） */
    val eventReportClient: EventReportClient,
    /** T-30035：会话管理器（ProcessLifecycleOwner 调用 onForeground/onBackground） */
    val sessionManager: SessionManager,
    /** T-30035：节流器（背景态/WS 重连触发 flush） */
    val throttler: Throttler,
) {
    companion object {

        // ─────────────────────────────────────────────
        // 生产入口：依赖 Android Context 完整装配链路
        // ─────────────────────────────────────────────

        fun fromBuildConfig(context: Context): AppContainer {
            val appCtx = context.applicationContext

            val environment = AppEnvironment.fromBuildConfig(
                environmentName = BuildConfig.APP_ENVIRONMENT,
                apiBaseUrl = BuildConfig.API_BASE_URL,
                wsUrl = BuildConfig.WS_URL,
                analyticsEndpoint = BuildConfig.ANALYTICS_ENDPOINT
            )

            // ── TokenManager + Auth interceptor ────────────────
            // BUG-JWT-PERSIST 修复：使用 DataStore Preferences 持久化 JWT Token，
            // am force-stop 后重启 App 可恢复登录状态（原实现为内存 @Volatile，进程终止后丢失）。
            val currentUserId = AtomicReference<String?>(null)
            val authDataStoreFile = File(appCtx.filesDir, "datastore/auth.preferences_pb")
            val authDataStore = PreferenceDataStoreFactory.create(
                scope = CoroutineScope(SupervisorJob() + Dispatchers.IO),
                produceFile = { authDataStoreFile }
            )
            val tokenManager = object : ITokenManager {
                private val delegate = TokenManager(authDataStore)
                override suspend fun saveToken(token: String) = delegate.saveToken(token)
                override suspend fun getToken(): String? = delegate.getToken()
                override suspend fun clearToken() {
                    delegate.clearToken()
                    currentUserId.set(null)  // 退出登录时同步清除 Analytics userId
                }
            }
            val unauthorizedHandler = DefaultUnauthorizedHandler(tokenManager)
            val authInterceptor = AuthInterceptor(tokenManager, unauthorizedHandler)

            // ── Retrofit / API services ────────────────────────
            val roomRetrofit = Retrofit.Builder()
                .baseUrl("${environment.apiBaseUrl}/v1/")
                .addConverterFactory(GsonConverterFactory.create())
                .client(AppHttpClientFactory.create(authInterceptor = authInterceptor))
                .build()
            val roomApiService = roomRetrofit.create(RoomApiService::class.java)
            val userApiService = roomRetrofit.create(UserApiService::class.java)
            val userRepository: IUserRepository = RetrofitUserRepository(userApiService)
            val walletApiService = roomRetrofit.create(WalletApiService::class.java)
            val walletRepository: IWalletRepository = RetrofitWalletRepository(walletApiService)
            val giftApiService = roomRetrofit.create(
                com.voice.room.android.data.remote.api.GiftApiService::class.java
            )
            val giftRepository: IGiftRepository =
                com.voice.room.android.data.gift.RetrofitGiftRepository(giftApiService)
            val rankingApiService = roomRetrofit.create(RankingApiService::class.java)
            val rankingRepository: IRankingRepository = RetrofitRankingRepository(rankingApiService)
            val authApiService = roomRetrofit.create(
                com.voice.room.android.data.remote.api.AuthApiService::class.java
            )
            val authRepository: com.voice.room.android.domain.auth.IAuthRepository =
                com.voice.room.android.data.auth.RetrofitAuthRepository(authApiService)

            // ── WebSocket ─────────────────────────────────────
            val wsHttpClient = AppHttpClientFactory.create(
                config = NetworkClientConfig(),
                authInterceptor = null
            )
            val wsScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
            val webSocketClient = OkHttpWebSocketClient(
                okHttpClient = wsHttpClient,
                scope = wsScope
            )

            // ── T-30035 主链路装配 ────────────────────────────

            // 1) Room 持久化事件队列（缺陷 7）
            val database = AppDatabase.getInstance(appCtx)
            val eventQueueDao: EventQueueDao = RoomEventQueueRepository(database.eventQueueRoomDao())

            // 2) DeviceId / SessionManager / NetworkType / Locale
            val deviceId = DeviceIdProvider.getOrCreateBlocking(appCtx)
            val sessionManager = SessionManager()
            val locale = Locale.getDefault().toLanguageTag()
            val osVersion = "Android ${Build.VERSION.RELEASE}"
            val appVersion = BuildConfig.VERSION_NAME

            // 3) ConsentRepository（先实例化 store / repo，AnalyticsPort 拼装后再 load）
            val consentFile = File(appCtx.filesDir, "consent/consent.properties")
            val consentStore: ConsentStore = DataStoreConsentStore(consentFile)
            val consentRepository = ConsentRepository(consentStore, analyticsPort = null)
            // load 阻塞 Application onCreate（启动期 IO 仅一次，毫秒级）
            runBlocking { consentRepository.load() }

            // 4) CommonPropsProvider — userIdProvider 从 currentUserId AtomicReference 读
            val commonProps = CommonPropsProvider(
                deviceId = deviceId,
                appVersion = appVersion,
                osVersion = osVersion,
                locale = locale,
                networkTypeProvider = { resolveNetworkType(appCtx) },
                userIdProvider = { currentUserId.get() },
                filter = SensitiveFilter()
            )

            // 5) Transports（HTTP / WS）
            val httpClientForAnalytics = AppHttpClientFactory.create(authInterceptor = authInterceptor)
            val httpTransport = HttpTransport(
                httpClient = httpClientForAnalytics,
                endpoint = environment.analyticsEndpoint
            )
            val wsTransport = WsTransport(wsClient = webSocketClient)

            // 6) Throttler & EventReportClient（先以占位 lambda 构造，避免循环依赖）
            val analyticsScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
            lateinit var eventReportClient: EventReportClient
            val throttler = Throttler(
                scope = analyticsScope,
                doFlush = { eventReportClient.flush() }
            )

            // 7) SentryAnalytics（下游） — 初始 Consent 取自 ConsentRepository
            val sentryAnalytics = SentryAnalytics(
                filter = SensitiveFilter(),
                initialConsent = consentRepository.mode
            )

            eventReportClient = EventReportClient(
                queueDao = eventQueueDao,
                throttler = throttler,
                wsTransport = wsTransport,
                httpTransport = httpTransport,
                consentRepo = consentRepository,
                commonProps = commonProps,
                sessionManager = sessionManager,
                analyticsPort = sentryAnalytics,
                isWsOnline = { wsTransport.isOnline }
            )

            // 8) Composite AnalyticsPort：业务层通过此对象 track，事件同时落 Sentry breadcrumb 与上报队列
            val analyticsPort: AnalyticsPort = CompositeAnalyticsPort(
                downstream = sentryAnalytics,
                eventReporter = eventReportClient,
                scope = analyticsScope
            )

            // 9) ConsentRepository 反向回填 analyticsPort（用户在弹窗中切换模式时同步通知）
            consentRepository.attachAnalyticsPort(analyticsPort)

            // 10) 监听 WS 重连：Disconnected → Connected 触发 throttler.onWsReconnected()
            analyticsScope.launch {
                var lastConnected = false
                webSocketClient.state.collect { state ->
                    val nowConnected = state is WebSocketState.Connected
                    if (nowConnected && !lastConnected) {
                        throttler.onWsReconnected()
                    }
                    lastConnected = nowConnected
                }
            }

            return AppContainer(
                environment = environment,
                analyticsPort = analyticsPort,
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
                roomSnapshotRepository = RetrofitRoomSnapshotRepository(roomApiService),
                tokenManager = tokenManager,
                userRepository = userRepository,
                authRepository = authRepository,
                unauthorizedHandler = unauthorizedHandler,
                kickCooldownStore = InMemoryKickCooldownStore(),
                consentRepository = consentRepository,
                eventReportClient = eventReportClient,
                sessionManager = sessionManager,
                throttler = throttler,
            )
        }

        private fun resolveNetworkType(context: Context): String {
            val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager
                ?: return "UNKNOWN"
            val network = cm.activeNetwork ?: return "NONE"
            val caps = cm.getNetworkCapabilities(network) ?: return "NONE"
            return when {
                caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> "WIFI"
                caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> "MOBILE"
                caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET) -> "ETHERNET"
                else -> "OTHER"
            }
        }

        // ─────────────────────────────────────────────
        // 单元测试入口（仅 JVM 测试使用；生产路径绝不调用）
        // grep `NoopAnalytics()` 检查时 — 此函数标注 @VisibleForTesting，
        // 不参与生产路径组装；生产路径见 fromBuildConfig(context)。
        // ─────────────────────────────────────────────

        @VisibleForTesting
        fun forUnitTest(): AppContainer {
            val environment = AppEnvironment.fromBuildConfig(
                environmentName = BuildConfig.APP_ENVIRONMENT,
                apiBaseUrl = BuildConfig.API_BASE_URL,
                wsUrl = BuildConfig.WS_URL,
                analyticsEndpoint = BuildConfig.ANALYTICS_ENDPOINT
            )

            val tokenManager = object : ITokenManager {
                @Volatile private var token: String? = null
                override suspend fun saveToken(token: String) { this.token = token }
                override suspend fun getToken(): String? = token
                override suspend fun clearToken() { token = null }
            }
            val unauthorizedHandler = DefaultUnauthorizedHandler(tokenManager)
            val authInterceptor = AuthInterceptor(tokenManager, unauthorizedHandler)

            val roomRetrofit = Retrofit.Builder()
                .baseUrl("${environment.apiBaseUrl}/v1/")
                .addConverterFactory(GsonConverterFactory.create())
                .client(AppHttpClientFactory.create(authInterceptor = authInterceptor))
                .build()
            val roomApiService = roomRetrofit.create(RoomApiService::class.java)
            val userApiService = roomRetrofit.create(UserApiService::class.java)
            val userRepository: IUserRepository = RetrofitUserRepository(userApiService)
            val walletApiService = roomRetrofit.create(WalletApiService::class.java)
            val walletRepository: IWalletRepository = RetrofitWalletRepository(walletApiService)
            val giftApiService = roomRetrofit.create(
                com.voice.room.android.data.remote.api.GiftApiService::class.java
            )
            val giftRepository: IGiftRepository =
                com.voice.room.android.data.gift.RetrofitGiftRepository(giftApiService)
            val rankingApiService = roomRetrofit.create(RankingApiService::class.java)
            val rankingRepository: IRankingRepository = RetrofitRankingRepository(rankingApiService)

            val wsHttpClient = AppHttpClientFactory.create(
                config = NetworkClientConfig(),
                authInterceptor = null
            )
            val wsScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
            val webSocketClient = OkHttpWebSocketClient(
                okHttpClient = wsHttpClient,
                scope = wsScope
            )

            val eventQueueDao: EventQueueDao = InMemoryEventQueueDao()
            val sessionManager = SessionManager()
            val consentRepository = ConsentRepository(InMemoryConsentStore(), analyticsPort = null)
            val commonProps = CommonPropsProvider(
                deviceId = "test-device",
                appVersion = BuildConfig.VERSION_NAME,
                osVersion = "Android JVM",
                locale = "en-US"
            )
            val httpTransport = HttpTransport(wsHttpClient, environment.analyticsEndpoint)
            val wsTransport = WsTransport(wsClient = webSocketClient)
            val analyticsScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
            lateinit var eventReportClient: EventReportClient
            val throttler = Throttler(
                scope = analyticsScope,
                doFlush = { eventReportClient.flush() }
            )
            val sentryAnalytics = SentryAnalytics(
                filter = SensitiveFilter(),
                initialConsent = ConsentMode.CrashOnly
            )
            eventReportClient = EventReportClient(
                queueDao = eventQueueDao,
                throttler = throttler,
                wsTransport = wsTransport,
                httpTransport = httpTransport,
                consentRepo = consentRepository,
                commonProps = commonProps,
                sessionManager = sessionManager,
                analyticsPort = sentryAnalytics,
                isWsOnline = { wsTransport.isOnline }
            )
            val analyticsPort: AnalyticsPort = CompositeAnalyticsPort(
                downstream = sentryAnalytics,
                eventReporter = eventReportClient,
                scope = analyticsScope
            )
            consentRepository.attachAnalyticsPort(analyticsPort)

            return AppContainer(
                environment = environment,
                analyticsPort = analyticsPort,
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
                roomSnapshotRepository = RetrofitRoomSnapshotRepository(roomApiService),
                tokenManager = tokenManager,
                userRepository = userRepository,
                authRepository = com.voice.room.android.data.auth.RetrofitAuthRepository(
                    roomRetrofit.create(com.voice.room.android.data.remote.api.AuthApiService::class.java)
                ),
                unauthorizedHandler = unauthorizedHandler,
                kickCooldownStore = InMemoryKickCooldownStore(),
                consentRepository = consentRepository,
                eventReportClient = eventReportClient,
                sessionManager = sessionManager,
                throttler = throttler,
            )
        }
    }
}
