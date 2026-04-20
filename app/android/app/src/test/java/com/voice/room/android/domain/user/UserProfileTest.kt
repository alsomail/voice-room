package com.voice.room.android.domain.user

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertNotNull
import org.junit.Test

/**
 * TDD 单元测试 — UserProfile 领域模型
 *
 * 覆盖范围：
 * 1. 所有字段（id, phone, nickname, avatar, coinBalance, vipLevel, createdAt）正确持有
 * 2. avatar 可以为 null
 * 3. coinBalance 类型为 Long（可保存大整数）
 * 4. data class 的 copy() 不改变其他字段
 * 5. 相同字段内容的两个 UserProfile 实例应相等（data class 语义）
 */
class UserProfileTest {

    private val sample = UserProfile(
        id = "550e8400-e29b-41d4-a716-446655440000",
        phone = "+966512345678",
        nickname = "User_a1b2",
        avatar = "https://cdn.example.com/avatars/xxx.jpg",
        coinBalance = 1000L,
        vipLevel = 2,
        createdAt = "2026-04-17T00:00:00Z"
    )

    // ─────────────────────────────────────────────
    // 1. 字段持有
    // ─────────────────────────────────────────────

    @Test
    fun `UserProfile holds all fields correctly`() {
        assertEquals("550e8400-e29b-41d4-a716-446655440000", sample.id)
        assertEquals("+966512345678", sample.phone)
        assertEquals("User_a1b2", sample.nickname)
        assertEquals("https://cdn.example.com/avatars/xxx.jpg", sample.avatar)
        assertEquals(1000L, sample.coinBalance)
        assertEquals(2, sample.vipLevel)
        assertEquals("2026-04-17T00:00:00Z", sample.createdAt)
    }

    // ─────────────────────────────────────────────
    // 2. avatar 可为 null
    // ─────────────────────────────────────────────

    @Test
    fun `UserProfile avatar can be null`() {
        val profileWithoutAvatar = sample.copy(avatar = null)
        assertNull(profileWithoutAvatar.avatar)
    }

    @Test
    fun `UserProfile avatar can be non-null`() {
        assertNotNull(sample.avatar)
    }

    // ─────────────────────────────────────────────
    // 3. coinBalance 类型为 Long
    // ─────────────────────────────────────────────

    @Test
    fun `UserProfile coinBalance is Long type and accepts large values`() {
        val richUser = sample.copy(coinBalance = Long.MAX_VALUE)
        assertEquals(Long.MAX_VALUE, richUser.coinBalance)
    }

    @Test
    fun `UserProfile coinBalance zero is valid for new user`() {
        val newUser = sample.copy(coinBalance = 0L)
        assertEquals(0L, newUser.coinBalance)
    }

    // ─────────────────────────────────────────────
    // 4. copy() 不影响未指定字段
    // ─────────────────────────────────────────────

    @Test
    fun `UserProfile copy changes only specified field`() {
        val updated = sample.copy(nickname = "NewNickname")
        assertEquals("NewNickname", updated.nickname)
        // 其他字段不变
        assertEquals(sample.id, updated.id)
        assertEquals(sample.phone, updated.phone)
        assertEquals(sample.avatar, updated.avatar)
        assertEquals(sample.coinBalance, updated.coinBalance)
        assertEquals(sample.vipLevel, updated.vipLevel)
        assertEquals(sample.createdAt, updated.createdAt)
    }

    // ─────────────────────────────────────────────
    // 5. data class 相等语义
    // ─────────────────────────────────────────────

    @Test
    fun `two UserProfile instances with same fields are equal`() {
        val copy = sample.copy()
        assertEquals(sample, copy)
    }

    @Test
    fun `two UserProfile instances with different id are not equal`() {
        val other = sample.copy(id = "different-uuid")
        assert(sample != other) { "Different id should produce different instances" }
    }

    // ─────────────────────────────────────────────
    // 6. vipLevel 边界值
    // ─────────────────────────────────────────────

    @Test
    fun `UserProfile vipLevel zero is valid`() {
        val freeUser = sample.copy(vipLevel = 0)
        assertEquals(0, freeUser.vipLevel)
    }

    @Test
    fun `UserProfile vipLevel positive value is valid`() {
        val vipUser = sample.copy(vipLevel = 5)
        assertEquals(5, vipUser.vipLevel)
    }
}
