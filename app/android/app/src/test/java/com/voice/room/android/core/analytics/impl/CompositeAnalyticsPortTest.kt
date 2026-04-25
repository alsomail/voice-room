package com.voice.room.android.core.analytics.impl

import com.voice.room.android.core.analytics.AnalyticsPort
import com.voice.room.android.core.analytics.ConsentMode
import com.voice.room.android.core.analytics.EventReportClient
import com.voice.room.android.core.analytics.context.CommonPropsProvider
import com.voice.room.android.core.analytics.queue.EventQueueEntity
import com.voice.room.android.core.analytics.queue.InMemoryEventQueueDao
import com.voice.room.android.core.analytics.session.SessionManager
import com.voice.room.android.core.analytics.throttle.Throttler
import com.voice.room.android.core.analytics.transport.SendOutcome
import com.voice.room.android.core.analytics.transport.Transport
import com.voice.room.android.core.consent.ConsentRepository
import com.voice.room.android.core.consent.InMemoryConsentStore
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Test

/**
 * [CompositeAnalyticsPort] 单元测试（T-30035 / R1 批 2 缺陷 2）
 */
@OptIn(ExperimentalCoroutinesApi::class)
class CompositeAnalyticsPortTest {

    private class StubTransport : Transport {
        override suspend fun send(batch: List<EventQueueEntity>): Result<SendOutcome> =
            Result.success(SendOutcome(batch.map { it.id }))
    }

    private class RecordingAnalyticsPort : AnalyticsPort {
        val tracks = mutableListOf<Pair<String, Map<String, Any?>>>()
        val setUsers = mutableListOf<String?>()
        val captures = mutableListOf<Throwable>()
        val consents = mutableListOf<ConsentMode>()
        override fun track(event: String, properties: Map<String, Any?>) { tracks += event to properties }
        override fun setUser(userId: String?, traits: Map<String, Any?>) { setUsers += userId }
        override fun captureException(throwable: Throwable, extras: Map<String, Any?>) { captures += throwable }
        override fun setConsent(mode: ConsentMode) { consents += mode }
    }

    private fun buildClient(consentRepo: ConsentRepository, scope: TestScope): EventReportClient {
        val queue = InMemoryEventQueueDao()
        val transport = StubTransport()
        lateinit var client: EventReportClient
        val throttler = Throttler(scope = scope, doFlush = { client.flush() })
        val commonProps = CommonPropsProvider(
            deviceId = "d", appVersion = "1", osVersion = "A", locale = "en"
        )
        client = EventReportClient(
            queueDao = queue,
            throttler = throttler,
            wsTransport = transport,
            httpTransport = transport,
            consentRepo = consentRepo,
            commonProps = commonProps,
            sessionManager = SessionManager(),
            analyticsPort = null,
            isWsOnline = { false }
        )
        return client
    }

    @Test
    fun `track fans out to downstream and event reporter`() = runTest {
        val downstream = RecordingAnalyticsPort()
        val consentRepo = ConsentRepository(InMemoryConsentStore()).apply { saveConsent(ConsentMode.All) }
        val client = buildClient(consentRepo, this)
        val composite = CompositeAnalyticsPort(downstream, client, this)

        composite.track("login_verify_success", mapOf("is_new_user" to true))
        advanceUntilIdle()

        assertEquals(1, downstream.tracks.size)
        assertEquals("login_verify_success", downstream.tracks[0].first)
    }

    @Test
    fun `setUser captureException setConsent delegate to downstream`() = runTest {
        val downstream = RecordingAnalyticsPort()
        val consentRepo = ConsentRepository(InMemoryConsentStore())
        val client = buildClient(consentRepo, this)
        val composite = CompositeAnalyticsPort(downstream, client, this)

        composite.setUser("u1", mapOf("k" to "v"))
        composite.captureException(IllegalStateException("x"))
        composite.setConsent(ConsentMode.All)

        assertEquals(listOf<String?>("u1"), downstream.setUsers)
        assertEquals(1, downstream.captures.size)
        assertNotNull(downstream.captures[0])
        assertEquals(listOf(ConsentMode.All), downstream.consents)
    }
}
