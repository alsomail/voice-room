package com.voice.room.android.feature.main

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Test

/**
 * MainTab 枚举 JVM 单元测试 (T-30020)
 *
 * 测试 Tab 枚举的 route、labelEn、labelAr、testTag 属性，
 * 以及条目数量和唯一性。
 */
class MainTabTest {

    // ── 条目数量 ─────────────────────────────────────────
    @Test
    fun `MainTab has exactly 3 entries`() {
        assertEquals(3, MainTab.entries.size)
    }

    // ── ROOMS 属性 ───────────────────────────────────────
    @Test
    fun `ROOMS route is main_rooms`() {
        assertEquals("main/rooms", MainTab.ROOMS.route)
    }

    @Test
    fun `ROOMS labelEn is Rooms`() {
        assertEquals("Rooms", MainTab.ROOMS.labelEn)
    }

    @Test
    fun `ROOMS labelAr is correct Arabic`() {
        assertEquals("الغرف", MainTab.ROOMS.labelAr)
    }

    @Test
    fun `ROOMS testTag is tab_rooms`() {
        assertEquals("tab_rooms", MainTab.ROOMS.testTag)
    }

    // ── MESSAGES 属性 ────────────────────────────────────
    @Test
    fun `MESSAGES route is main_messages`() {
        assertEquals("main/messages", MainTab.MESSAGES.route)
    }

    @Test
    fun `MESSAGES labelEn is Messages`() {
        assertEquals("Messages", MainTab.MESSAGES.labelEn)
    }

    @Test
    fun `MESSAGES labelAr is correct Arabic`() {
        assertEquals("الرسائل", MainTab.MESSAGES.labelAr)
    }

    @Test
    fun `MESSAGES testTag is tab_messages`() {
        assertEquals("tab_messages", MainTab.MESSAGES.testTag)
    }

    // ── PROFILE 属性 ─────────────────────────────────────
    @Test
    fun `PROFILE route is main_profile`() {
        assertEquals("main/profile", MainTab.PROFILE.route)
    }

    @Test
    fun `PROFILE labelEn is Me`() {
        assertEquals("Me", MainTab.PROFILE.labelEn)
    }

    @Test
    fun `PROFILE labelAr is correct Arabic`() {
        assertEquals("حسابي", MainTab.PROFILE.labelAr)
    }

    @Test
    fun `PROFILE testTag is tab_profile`() {
        assertEquals("tab_profile", MainTab.PROFILE.testTag)
    }

    // ── 路由唯一性 ───────────────────────────────────────
    @Test
    fun `all routes are unique`() {
        val routes = MainTab.entries.map { it.route }
        assertEquals(routes.size, routes.distinct().size)
    }

    // ── testTag 唯一性 ──────────────────────────────────
    @Test
    fun `all testTags are unique`() {
        val tags = MainTab.entries.map { it.testTag }
        assertEquals(tags.size, tags.distinct().size)
    }

    // ── 顺序验证 ────────────────────────────────────────
    @Test
    fun `entries order is ROOMS - MESSAGES - PROFILE`() {
        val entries = MainTab.entries
        assertEquals(MainTab.ROOMS, entries[0])
        assertEquals(MainTab.MESSAGES, entries[1])
        assertEquals(MainTab.PROFILE, entries[2])
    }

    // ── icon 非空（JVM 环境下 ImageVector 可用）──────────
    @Test
    fun `all tabs have non-null icon`() {
        MainTab.entries.forEach { tab ->
            assertNotEquals("${tab.name} icon should not be null", null, tab.icon)
        }
    }
}
