package com.voice.room.android.data.local

import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

/**
 * TDD 单元测试 — TokenManager (BUG-JWT-PERSIST)
 *
 * 验收标准：DataStore 实现在进程生命周期内正确持久化 JWT Token，
 * 消除原始 @Volatile in-memory 实现在 am force-stop 后丢失 Token 的问题。
 *
 * TM01: getToken 在未保存任何 token 时返回 null
 * TM02: saveToken 保存后 getToken 应返回同一值
 * TM03: clearToken 后 getToken 应返回 null
 * TM04: 二次 saveToken 应覆盖旧值
 * TM05: 空字符串 saveToken → getToken 能区分"已保存空串"与"未保存"（DataStore 存空串）
 * TM06: clearToken 在无 token 时调用不应抛出异常
 * TM07: saveToken 支持含特殊字符的 JWT（Base64 字母表 + . 和 -）
 */
@OptIn(ExperimentalCoroutinesApi::class)
class TokenManagerTest {

    @get:Rule
    val tempFolder = TemporaryFolder()

    /**
     * 每个测试用例使用独立的临时文件，避免 DataStore 单文件单实例约束的干扰。
     */
    private fun createTokenManager(fileName: String = "test_auth.preferences_pb"): TokenManager {
        val testDispatcher = UnconfinedTestDispatcher()
        val scope = CoroutineScope(testDispatcher)
        val file = tempFolder.newFile(fileName)
        val dataStore = PreferenceDataStoreFactory.create(
            scope = scope,
            produceFile = { file }
        )
        return TokenManager(dataStore)
    }

    // ── TM01: 未保存时 getToken 返回 null ────────────────────────────────────

    /**
     * [RED] → 原始 NoOp/volatile 实现同样返回 null，但该测试确立了
     * DataStore 实现的基准：冷启动时没有已保存的 token。
     */
    @Test
    fun `TM01 getToken returns null when no token has been saved`() = runTest {
        val tokenManager = createTokenManager("tm01.preferences_pb")

        val result = tokenManager.getToken()

        assertNull("getToken 应在未保存 token 时返回 null", result)
    }

    // ── TM02: saveToken → getToken 往返一致 ─────────────────────────────────

    /**
     * [RED → GREEN] BUG-JWT-PERSIST 核心修复：
     * DataStore 实现必须将 token 写入文件，getToken 才能读回；
     * 原始 @Volatile 内存实现在同一进程内也能通过，但 am force-stop 后丢失。
     */
    @Test
    fun `TM02 saveToken stores value and getToken returns it`() = runTest {
        val tokenManager = createTokenManager("tm02.preferences_pb")

        tokenManager.saveToken("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9")

        assertEquals(
            "getToken 应返回 saveToken 写入的值",
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
            tokenManager.getToken()
        )
    }

    // ── TM03: clearToken 后 getToken 返回 null ───────────────────────────────

    @Test
    fun `TM03 clearToken removes saved token and getToken returns null`() = runTest {
        val tokenManager = createTokenManager("tm03.preferences_pb")
        tokenManager.saveToken("jwt-to-be-cleared")

        tokenManager.clearToken()

        assertNull("clearToken 后 getToken 应返回 null", tokenManager.getToken())
    }

    // ── TM04: 二次 saveToken 覆盖旧值 ───────────────────────────────────────

    @Test
    fun `TM04 second saveToken overwrites first token`() = runTest {
        val tokenManager = createTokenManager("tm04.preferences_pb")
        tokenManager.saveToken("old-token-v1")

        tokenManager.saveToken("new-token-v2")

        assertEquals(
            "二次调用 saveToken 应覆盖旧 token",
            "new-token-v2",
            tokenManager.getToken()
        )
    }

    // ── TM05: 保存空字符串与未保存的行为对比 ─────────────────────────────────

    /**
     * DataStore stringPreferencesKey 可区分「有值（空串）」与「无值（null）」。
     * 明确记录 TokenManager 保存空串后 getToken 返回空串（非 null），
     * 避免上层误判为"未登录"。
     */
    @Test
    fun `TM05 saving empty string token returns empty string not null`() = runTest {
        val tokenManager = createTokenManager("tm05.preferences_pb")

        tokenManager.saveToken("")

        // DataStore 存储空串后读回为空串，与未保存（null）可区分
        assertEquals(
            "保存空串后 getToken 应返回空串（非 null）",
            "",
            tokenManager.getToken()
        )
    }

    // ── TM06: clearToken 在空 store 上调用不抛异常 ──────────────────────────

    @Test
    fun `TM06 clearToken on empty store does not throw and getToken remains null`() = runTest {
        val tokenManager = createTokenManager("tm06.preferences_pb")

        // 不预先保存任何 token，直接 clear
        tokenManager.clearToken()

        assertNull("clearToken 后空 store 的 getToken 仍应返回 null", tokenManager.getToken())
    }

    // ── TM07: JWT 特殊字符（Base64url + 句点）正确持久化 ────────────────────

    @Test
    fun `TM07 saveToken persists JWT with dots and Base64url characters`() = runTest {
        val tokenManager = createTokenManager("tm07.preferences_pb")
        // 真实 JWT 格式：header.payload.signature（含 - 和 _ 的 Base64url 字符集）
        val realJwt =
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9" +
            ".eyJzdWIiOiJ1c2VyLTEyMyIsImlhdCI6MTcwMDAwMDAwMH0" +
            ".abc-def_GHI123JKL456MNO789"

        tokenManager.saveToken(realJwt)

        assertEquals(
            "含特殊字符的 JWT 应完整持久化并读回",
            realJwt,
            tokenManager.getToken()
        )
    }
}
