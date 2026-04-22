package com.voice.room.android.core.analytics.eventreport

import com.voice.room.android.core.analytics.AnalyticsPort
import com.voice.room.android.core.analytics.ConsentMode
import com.voice.room.android.core.analytics.EventReportClient
import com.voice.room.android.core.analytics.context.CommonPropsProvider
import com.voice.room.android.core.analytics.privacy.SensitiveFilter
import com.voice.room.android.core.analytics.queue.EventQueueDao
import com.voice.room.android.core.analytics.queue.EventQueueEntity
import com.voice.room.android.core.analytics.queue.InMemoryEventQueueDao
import com.voice.room.android.core.analytics.session.Clock
import com.voice.room.android.core.analytics.session.SessionManager
import com.voice.room.android.core.analytics.throttle.Throttler
import com.voice.room.android.core.analytics.transport.SendOutcome
import com.voice.room.android.core.analytics.transport.Transport
import com.voice.room.android.core.consent.ConsentRepository
import com.voice.room.android.core.consent.InMemoryConsentStore
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestCoroutineScheduler
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test

/**
 * EventReportClient TDD 测试套件（T-30035）
 *
 * 覆盖验收用例 E35-01 ~ E35-08, E35-10, E35-11, E35-12
 */
@OptIn(ExperimentalCoroutinesApi::class)
class EventReportClientTest {

    // ── Fakes ──────────────────────────────────────────────────────────────

    /** 可控 Transport：记录发送的批次，可模拟成功/失败 */
    private class FakeTransport(
        private val shouldSucceed: Boolean = true
    ) : Transport {
        val sentBatches = mutableListOf<List<EventQueueEntity>>()
        var callCount = 0

        override suspend fun send(batch: List<EventQueueEntity>): Result<SendOutcome> {
            callCount++
            sentBatches.add(batch.toList())
            return if (shouldSucceed) {
                Result.success(SendOutcome(batch.map { it.id }))
            } else {
                Result.failure(RuntimeException("Simulated network failure"))
            }
        }
    }

    /** 固定时钟 */
    private class FakeClock(var timeMs: Long = 0L) : Clock {
        override fun now(): Long = timeMs
    }

    /** 记录所有 captureException 调用的 AnalyticsPort */
    private class SpyAnalyticsPort : AnalyticsPort {
        val capturedExceptions = mutableListOf<Throwable>()
        var consentModeSet: ConsentMode? = null

        override fun track(event: String, properties: Map<String, Any?>) = Unit
        override fun setUser(userId: String?, traits: Map<String, Any?>) = Unit
        override fun captureException(throwable: Throwable, extras: Map<String, Any?>) {
            capturedExceptions.add(throwable)
        }
        override fun setConsent(mode: ConsentMode) { consentModeSet = mode }
    }

    // ── Test Fixtures ──────────────────────────────────────────────────────

    private lateinit var scheduler: TestCoroutineScheduler
    private lateinit var testScope: TestScope
    private lateinit var queueDao: InMemoryEventQueueDao
    private lateinit var wsTransport: FakeTransport
    private lateinit var httpTransport: FakeTransport
    private lateinit var fakeClock: FakeClock
    private lateinit var sessionManager: SessionManager
    private lateinit var spyAnalytics: SpyAnalyticsPort
    private lateinit var commonProps: CommonPropsProvider

    @Before
    fun setUp() {
        scheduler = TestCoroutineScheduler()
        testScope = TestScope(StandardTestDispatcher(scheduler))
        queueDao = InMemoryEventQueueDao()
        wsTransport = FakeTransport(shouldSucceed = true)
        httpTransport = FakeTransport(shouldSucceed = true)
        fakeClock = FakeClock(0L)
        sessionManager = SessionManager(clock = fakeClock, sessionTimeoutMs = 30_000L)
        spyAnalytics = SpyAnalyticsPort()
        commonProps = CommonPropsProvider(
            deviceId = "test-device-001",
            appVersion = "0.1.0",
            osVersion = "14",
            locale = "zh-CN",
            filter = SensitiveFilter()
        )
    }

    private fun buildClient(
        mode: ConsentMode = ConsentMode.All,
        wsOnline: Boolean = true,
        ws: Transport = wsTransport,
        http: Transport = httpTransport,
        dao: EventQueueDao = queueDao
    ): Pair<EventReportClient, Throttler> {
        val consentStore = InMemoryConsentStore()
        // 同步预填充 store（InMemoryConsentStore 无真实 IO，runBlocking 安全）
        kotlinx.coroutines.runBlocking { consentStore.save(mode) }
        val repo = ConsentRepository(consentStore, spyAnalytics)
        // load 使 isSet=true，_mode 更新
        kotlinx.coroutines.runBlocking { repo.load() }

        val clientRef = arrayOfNulls<EventReportClient>(1)

        val throttler = Throttler(
            batchSize = 8,
            flushIntervalMs = 2L * 60_000L,
            clock = fakeClock,
            scope = testScope
        ) {
            clientRef[0]?.flush()
        }

        val client = EventReportClient(
            queueDao = dao,
            throttler = throttler,
            wsTransport = ws,
            httpTransport = http,
            consentRepo = repo,
            commonProps = commonProps,
            sessionManager = sessionManager,
            analyticsPort = spyAnalytics,
            isWsOnline = { wsOnline }
        )
        clientRef[0] = client
        return Pair(client, throttler)
    }

    // ── E35-01: track("x") 写入 Room 队列 ────────────────────────────────

    @Test
    fun `E35-01 track event writes entity to queue`() = testScope.runTest {
        val (client, _) = buildClient()

        client.track("login_success", mapOf("user_id" to "u123"))

        val size = queueDao.size()
        assertEquals("队列应有 1 条事件", 1, size)

        val oldest = queueDao.getOldest(1)
        assertEquals("login_success", oldest[0].eventName)
        assertTrue("应包含 session_id", oldest[0].sessionId.isNotEmpty())
        assertTrue("propertiesJson 不为空", oldest[0].propertiesJson.isNotEmpty())
    }

    // ── E35-02: 队列 ≥8 条后立即 flush ────────────────────────────────────

    @Test
    fun `E35-02 queue reaching 8 items triggers immediate flush`() = testScope.runTest {
        val (client, _) = buildClient(wsOnline = true)

        repeat(8) { i ->
            client.track("event_$i")
        }
        advanceUntilIdle()

        // flush 后队列应被清空（所有 8 条成功上报）
        assertEquals("flush 后队列应为空", 0, queueDao.size())
        assertEquals("WsTransport 应被调用 1 次", 1, wsTransport.callCount)
        assertEquals("批次应包含 8 条事件", 8, wsTransport.sentBatches[0].size)
    }

    // ── E35-03: 队列 3 条 + 经过 2min → flush ────────────────────────────

    @Test
    fun `E35-03 queue with 3 items flushes after 2 minutes`() = testScope.runTest {
        val (client, throttler) = buildClient(wsOnline = true)

        // 插入 3 条（不足 8 条，不触发 batch flush）
        repeat(3) { i ->
            client.track("event_$i")
        }
        advanceUntilIdle()
        assertEquals("3 条不足以触发 flush", 3, queueDao.size())

        // 时间推进 2min + 1ms
        fakeClock.timeMs = 2L * 60_000L + 1L

        // 再插入 1 条触发 notify（模拟定时器检查）
        client.track("event_trigger")
        advanceUntilIdle()

        // 应已 flush
        assertEquals("时间条件满足后队列应被清空", 0, queueDao.size())
        assertTrue("WsTransport 应被调用至少 1 次", wsTransport.callCount >= 1)
    }

    // ── E35-04: 队列 >1000 条时淘汰最旧 ──────────────────────────────────

    @Test
    fun `E35-04 queue evicts oldest when exceeding 1000 capacity`() = testScope.runTest {
        // 不触发 flush（使用永远失败的 transport）
        val failTransport = FakeTransport(shouldSucceed = false)
        val consentStore = InMemoryConsentStore()
        kotlinx.coroutines.runBlocking { consentStore.save(ConsentMode.All) }
        val repo = ConsentRepository(consentStore)
        kotlinx.coroutines.runBlocking { repo.load() }

        val client = EventReportClient(
            queueDao = queueDao,
            throttler = Throttler(
                batchSize = Int.MAX_VALUE, // 永不触发 batch flush
                flushIntervalMs = Long.MAX_VALUE,
                clock = fakeClock,
                scope = testScope
            ) { /* no-op */ },
            wsTransport = failTransport,
            httpTransport = failTransport,
            consentRepo = repo,
            commonProps = commonProps,
            sessionManager = sessionManager,
            isWsOnline = { false }
        )

        // 插入 1001 条
        repeat(1001) { i ->
            client.track("event_$i")
        }

        val finalSize = queueDao.size()
        assertEquals("容量应被限制在 1000", 1000, finalSize)

        // 最旧的事件（event_0）应被淘汰
        val allEvents = queueDao.getOldest(1000)
        val names = allEvents.map { it.eventName }.toSet()
        assertFalse("event_0 应被淘汰", names.contains("event_0"))
        assertTrue("event_1000 应保留", names.contains("event_1000"))
    }

    // ── E35-05: WS 在线：走 WsTransport ──────────────────────────────────

    @Test
    fun `E35-05 uses WsTransport when WS is online`() = testScope.runTest {
        val (client, _) = buildClient(wsOnline = true)

        repeat(8) { client.track("event_$it") }
        advanceUntilIdle()

        assertEquals("WS 在线时应使用 WsTransport", 1, wsTransport.callCount)
        assertEquals("HTTP 不应被调用", 0, httpTransport.callCount)
    }

    // ── E35-06: WS 离线：走 HttpTransport ────────────────────────────────

    @Test
    fun `E35-06 uses HttpTransport when WS is offline`() = testScope.runTest {
        val (client, _) = buildClient(wsOnline = false)

        repeat(8) { client.track("event_$it") }
        advanceUntilIdle()

        assertEquals("HTTP 应被调用（WS 离线）", 1, httpTransport.callCount)
        assertEquals("WS 不应被调用", 0, wsTransport.callCount)
    }

    // ── E35-07: 断网 5min 后恢复：缓存事件 100% 补报 ──────────────────

    @Test
    fun `E35-07 cached events are all reported after network recovers`() = testScope.runTest {
        val failingHttp = FakeTransport(shouldSucceed = false)
        val consentStore = InMemoryConsentStore()
        kotlinx.coroutines.runBlocking { consentStore.save(ConsentMode.All) }
        val repo = ConsentRepository(consentStore)
        kotlinx.coroutines.runBlocking { repo.load() }

        // 构建不触发 batch flush 的 throttler
        val noFlushThrottler = Throttler(
            batchSize = Int.MAX_VALUE,
            flushIntervalMs = Long.MAX_VALUE,
            clock = fakeClock,
            scope = testScope
        ) { /* no-op */ }

        val client = EventReportClient(
            queueDao = queueDao,
            throttler = noFlushThrottler,
            wsTransport = failingHttp,
            httpTransport = failingHttp,
            consentRepo = repo,
            commonProps = commonProps,
            sessionManager = sessionManager,
            isWsOnline = { false }
        )

        // 断网期间积累 100 条事件
        val eventCount = 100
        repeat(eventCount) { i ->
            client.track("cached_event_$i")
        }
        assertEquals("断网期间队列应有 $eventCount 条", eventCount, queueDao.size())

        // 网络恢复：手动 flush（用成功的 HTTP transport）
        val successHttp = FakeTransport(shouldSucceed = true)
        val recoveredClient = EventReportClient(
            queueDao = queueDao,
            throttler = noFlushThrottler,
            wsTransport = successHttp,
            httpTransport = successHttp,
            consentRepo = repo,
            commonProps = commonProps,
            sessionManager = sessionManager,
            isWsOnline = { false }
        )

        // flush 所有缓存（分批，每批 100 条）
        recoveredClient.flush()
        advanceUntilIdle()

        assertEquals("恢复后队列应清空", 0, queueDao.size())
        assertEquals("所有 $eventCount 条事件应被上报", eventCount,
            successHttp.sentBatches.flatten().size)
    }

    // ── E35-08: ConsentMode.CrashOnly → track() 立即返回，不入队 ─────────

    @Test
    fun `E35-08 CrashOnly mode discards track calls`() = testScope.runTest {
        val (client, _) = buildClient(mode = ConsentMode.CrashOnly)

        client.track("login_success")
        client.track("gift_send")

        assertEquals("CrashOnly 模式不入队", 0, queueDao.size())
    }

    @Test
    fun `E35-08b None mode also discards track calls`() = testScope.runTest {
        val (client, _) = buildClient(mode = ConsentMode.None)

        client.track("login_success")

        assertEquals("None 模式不入队", 0, queueDao.size())
    }

    // ── E35-10: 核心事件集成测试 ─────────────────────────────────────────

    @Test
    fun `E35-10 core events login_success gift_send_success gift_send_fail insufficient_balance are tracked`() =
        testScope.runTest {
            val (client, _) = buildClient(mode = ConsentMode.All, wsOnline = false)

            // 核心事件埋点
            client.track("login_success", mapOf("user_id" to "u999"))
            client.track("gift_send_success", mapOf("gift_id" to "g1", "amount" to 100))
            client.track("gift_send_fail", mapOf("reason" to "timeout"))
            client.track("insufficient_balance_dialog_shown", mapOf("required" to 500))

            assertEquals("4 个核心事件应全部入队", 4, queueDao.size())

            val events = queueDao.getOldest(10)
            val eventNames = events.map { it.eventName }
            assertContains("login_success", eventNames)
            assertContains("gift_send_success", eventNames)
            assertContains("gift_send_fail", eventNames)
            assertContains("insufficient_balance_dialog_shown", eventNames)
        }

    private fun assertContains(expected: String, list: List<String>) {
        assertTrue("期望包含 '$expected'，实际: $list", list.contains(expected))
    }

    // ── E35-11: 手机号 / JWT 字段被 SensitiveFilter 过滤 ─────────────────

    @Test
    fun `E35-11 phone number in properties is redacted`() = testScope.runTest {
        val (client, _) = buildClient()

        client.track("login_success", mapOf(
            "phone" to "+966512345678",
            "user_id" to "u123"
        ))

        val entity = queueDao.getOldest(1)[0]
        assertFalse(
            "手机号应被脱敏，不应出现在 propertiesJson 中",
            entity.propertiesJson.contains("+966512345678")
        )
        assertTrue(
            "脱敏占位符 *** 应在 propertiesJson 中",
            entity.propertiesJson.contains("***")
        )
    }

    @Test
    fun `E35-11b JWT token in properties is redacted`() = testScope.runTest {
        val (client, _) = buildClient()
        val jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1MTIzIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"

        client.track("login_success", mapOf("token" to jwt))

        val entity = queueDao.getOldest(1)[0]
        assertFalse(
            "JWT 应被脱敏",
            entity.propertiesJson.contains("eyJhbGciOiJIUzI1NiJ9")
        )
    }

    // ── E35-12: session_id 前台→后台30s→前台切换 ─────────────────────────

    @Test
    fun `E35-12 new session_id generated after 30s background`() = testScope.runTest {
        val (client, _) = buildClient()

        // 首次进前台
        fakeClock.timeMs = 0L
        sessionManager.onForeground()
        val firstSessionId = sessionManager.currentId

        client.track("event_before_bg")

        // 进入后台
        fakeClock.timeMs = 1_000L
        sessionManager.onBackground()

        // 时间推进超过 30s
        fakeClock.timeMs = 35_000L

        // 回到前台
        sessionManager.onForeground()
        val secondSessionId = sessionManager.currentId

        client.track("event_after_bg")

        // session_id 应不同
        assertNotEquals(
            "后台超 30s 后回前台应生成新 session_id",
            firstSessionId,
            secondSessionId
        )

        // 验证两个事件的 session_id 不同
        val events = queueDao.getOldest(10)
        assertEquals(2, events.size)
        assertNotEquals(events[0].sessionId, events[1].sessionId)
    }

    @Test
    fun `E35-12b session_id unchanged if background less than 30s`() = testScope.runTest {
        fakeClock.timeMs = 0L
        sessionManager.onForeground()
        val firstSessionId = sessionManager.currentId

        // 后台不足 30s
        fakeClock.timeMs = 1_000L
        sessionManager.onBackground()
        fakeClock.timeMs = 25_000L
        sessionManager.onForeground()

        assertEquals(
            "后台不足 30s，session_id 应保持不变",
            firstSessionId,
            sessionManager.currentId
        )
    }
}
