package com.voice.room.android.feature.main

import com.voice.room.android.R
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Test

/**
 * MainTab 枚举 JVM 单元测试 (T-30020 / 缺陷 #3 修复)
 *
 * 测试 Tab 枚举的 route、labelRes（@StringRes）、testTag 属性，
 * 以及条目数量和唯一性。
 *
 * 缺陷 #3 修复后，原 `labelEn` / `labelAr` 双字段被替换为单一的
 * `labelRes: Int`，由 `stringResource` 在 Composable 中按系统 Locale 解析。
 */
class MainTabTest {

    @Test
    fun `MainTab has exactly 3 entries`() {
        assertEquals(3, MainTab.entries.size)
    }

    // ── ROOMS ───────────────────────────────────────────
    @Test
    fun `ROOMS route is main_rooms`() {
        assertEquals("main/rooms", MainTab.ROOMS.route)
    }

    @Test
    fun `ROOMS labelRes points to tab_rooms string resource`() {
        assertEquals(R.string.tab_rooms, MainTab.ROOMS.labelRes)
    }

    @Test
    fun `ROOMS testTag is tab_rooms`() {
        assertEquals("tab_rooms", MainTab.ROOMS.testTag)
    }

    // ── MESSAGES ────────────────────────────────────────
    @Test
    fun `MESSAGES route is main_messages`() {
        assertEquals("main/messages", MainTab.MESSAGES.route)
    }

    @Test
    fun `MESSAGES labelRes points to tab_messages string resource`() {
        assertEquals(R.string.tab_messages, MainTab.MESSAGES.labelRes)
    }

    @Test
    fun `MESSAGES testTag is tab_messages`() {
        assertEquals("tab_messages", MainTab.MESSAGES.testTag)
    }

    // ── PROFILE ─────────────────────────────────────────
    @Test
    fun `PROFILE route is main_profile`() {
        assertEquals("main/profile", MainTab.PROFILE.route)
    }

    @Test
    fun `PROFILE labelRes points to tab_profile string resource`() {
        assertEquals(R.string.tab_profile, MainTab.PROFILE.labelRes)
    }

    @Test
    fun `PROFILE testTag is tab_profile`() {
        assertEquals("tab_profile", MainTab.PROFILE.testTag)
    }

    // ── 唯一性 ──────────────────────────────────────────
    @Test
    fun `all routes are unique`() {
        val routes = MainTab.entries.map { it.route }
        assertEquals(routes.size, routes.distinct().size)
    }

    @Test
    fun `all labelRes are unique`() {
        val labels = MainTab.entries.map { it.labelRes }
        assertEquals(labels.size, labels.distinct().size)
    }

    @Test
    fun `all testTags are unique`() {
        val tags = MainTab.entries.map { it.testTag }
        assertEquals(tags.size, tags.distinct().size)
    }

    // ── 顺序 ────────────────────────────────────────────
    @Test
    fun `entries order is ROOMS - MESSAGES - PROFILE`() {
        val entries = MainTab.entries
        assertEquals(MainTab.ROOMS, entries[0])
        assertEquals(MainTab.MESSAGES, entries[1])
        assertEquals(MainTab.PROFILE, entries[2])
    }

    @Test
    fun `all tabs have non-null icon`() {
        MainTab.entries.forEach { tab ->
            assertNotEquals("${tab.name} icon should not be null", null, tab.icon)
        }
    }
}
