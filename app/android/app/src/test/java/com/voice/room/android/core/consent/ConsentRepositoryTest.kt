package com.voice.room.android.core.consent

import com.voice.room.android.core.analytics.AnalyticsPort
import com.voice.room.android.core.analytics.ConsentMode
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Test

/**
 * ConsentRepository TDD 测试（T-30035）
 *
 * 覆盖验收用例 E35-09：隐私弹窗首次显示，选择后 DataStore 持久化
 */
class ConsentRepositoryTest {

    private class SpyAnalyticsPort : AnalyticsPort {
        var lastSetConsent: ConsentMode? = null
        override fun track(event: String, properties: Map<String, Any?>) = Unit
        override fun setUser(userId: String?, traits: Map<String, Any?>) = Unit
        override fun captureException(throwable: Throwable, extras: Map<String, Any?>) = Unit
        override fun setConsent(mode: ConsentMode) { lastSetConsent = mode }
    }

    // ── E35-09: 首次启动默认未设置 ────────────────────────────────────────

    @Test
    fun `E35-09a initial state is not set and defaults to CrashOnly`() = runTest {
        val store = InMemoryConsentStore()
        val repo = ConsentRepository(store)

        assertFalse("首次启动 isSet 应为 false", repo.isSet)
        assertEquals("默认模式应为 CrashOnly", ConsentMode.CrashOnly, repo.mode)
    }

    // ── E35-09: load 加载已持久化的 mode ─────────────────────────────────

    @Test
    fun `E35-09b load restores persisted consent mode`() = runTest {
        val store = InMemoryConsentStore()
        store.save(ConsentMode.All)

        val repo = ConsentRepository(store)
        repo.load()

        assertEquals("加载后 mode 应为 All", ConsentMode.All, repo.mode)
        assertTrue("isSet 应为 true", repo.isSet)
    }

    // ── E35-09: saveConsent 持久化并更新内存状态 ──────────────────────────

    @Test
    fun `E35-09c saveConsent persists to store and updates mode`() = runTest {
        val store = InMemoryConsentStore()
        val repo = ConsentRepository(store)

        repo.saveConsent(ConsentMode.All)

        assertEquals("mode 应更新为 All", ConsentMode.All, repo.mode)
        assertTrue("isSet 应为 true", repo.isSet)

        // 验证持久化
        val loaded = store.load()
        assertEquals("DataStore 中应保存 All", ConsentMode.All, loaded)
    }

    @Test
    fun `E35-09d saveConsent CrashOnly persists correctly`() = runTest {
        val store = InMemoryConsentStore()
        val repo = ConsentRepository(store)

        repo.saveConsent(ConsentMode.CrashOnly)

        assertEquals(ConsentMode.CrashOnly, repo.mode)
        assertEquals(ConsentMode.CrashOnly, store.load())
    }

    // ── E35-09: saveConsent 通知 AnalyticsPort ────────────────────────────

    @Test
    fun `E35-09e saveConsent notifies AnalyticsPort`() = runTest {
        val store = InMemoryConsentStore()
        val spy = SpyAnalyticsPort()
        val repo = ConsentRepository(store, spy)

        repo.saveConsent(ConsentMode.All)

        assertEquals(
            "应通知 AnalyticsPort.setConsent(All)",
            ConsentMode.All,
            spy.lastSetConsent
        )
    }

    // ── E35-09: 未设置时 load 不改变 isSet ──────────────────────────────

    @Test
    fun `E35-09f load with nothing persisted keeps isSet false`() = runTest {
        val store = InMemoryConsentStore()
        val repo = ConsentRepository(store)

        repo.load()

        assertFalse("空存储加载后 isSet 应仍为 false", repo.isSet)
    }

    // ── 重复保存不同 mode ────────────────────────────────────────────────

    @Test
    fun `saveConsent overwrites previous selection`() = runTest {
        val store = InMemoryConsentStore()
        val repo = ConsentRepository(store)

        repo.saveConsent(ConsentMode.CrashOnly)
        assertEquals(ConsentMode.CrashOnly, repo.mode)

        repo.saveConsent(ConsentMode.All)
        assertEquals("覆盖后 mode 应为 All", ConsentMode.All, repo.mode)
        assertEquals(ConsentMode.All, store.load())
    }
}
