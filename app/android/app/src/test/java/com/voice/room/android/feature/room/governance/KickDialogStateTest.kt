package com.voice.room.android.feature.room.governance

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * TDD 单元测试 — KickDialogState 纯状态测试（T-30041）
 *
 * 验收用例：
 *  KR41-01: 默认 selected = Harassment
 *  KR41-02: selected=Other, customText="" → canSubmit=false
 *  KR41-03: selected=Other, customText="xyz" → canSubmit=true
 *  KR41-04: submitting=true → canSubmit=false（无法重复提交）
 *  KR41-07: 预设原因 key 正确；Other reason = customText.trim()
 *
 * 测试策略：只测行为（输出），不测内部实现细节。
 */
class KickDialogStateTest {

    // ──────────────────────────────────────────────────────────────────────────
    // KR41-01: 默认 selected = Harassment
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `KR41-01 default state - selected is Harassment`() {
        val state = KickDialogState()
        assertEquals(KickReason.Harassment, state.selected)
    }

    @Test
    fun `KR41-01 default state - customText is empty`() {
        val state = KickDialogState()
        assertEquals("", state.customText)
    }

    @Test
    fun `KR41-01 default state - submitting is false`() {
        val state = KickDialogState()
        assertFalse(state.submitting)
    }

    @Test
    fun `KR41-01 default state - canSubmit is true for preset reason`() {
        val state = KickDialogState()
        assertTrue("Default Harassment reason should be submittable", state.canSubmit)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // KR41-02: selected=Other, customText="" → canSubmit=false
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `KR41-02 selected Other with empty customText - canSubmit is false`() {
        val state = KickDialogState(selected = KickReason.Other, customText = "")
        assertFalse("Other + empty customText should block submission", state.canSubmit)
    }

    @Test
    fun `KR41-02 selected Other with blank whitespace customText - canSubmit is false`() {
        val state = KickDialogState(selected = KickReason.Other, customText = "   ")
        assertFalse("Other + whitespace-only customText should block submission", state.canSubmit)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // KR41-03: selected=Other, customText="xyz" → canSubmit=true
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `KR41-03 selected Other with non-empty customText - canSubmit is true`() {
        val state = KickDialogState(selected = KickReason.Other, customText = "xyz")
        assertTrue("Other + non-empty customText should allow submission", state.canSubmit)
    }

    @Test
    fun `KR41-03 selected Other with trimmed non-empty customText - canSubmit is true`() {
        val state = KickDialogState(selected = KickReason.Other, customText = "  reasons  ")
        assertTrue("Other + padded non-empty customText should allow submission", state.canSubmit)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // KR41-04: submitting=true → canSubmit=false（无法重复提交）
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `KR41-04 submitting true with Harassment - canSubmit is false`() {
        val state = KickDialogState(selected = KickReason.Harassment, submitting = true)
        assertFalse("submitting=true should prevent re-submission for preset reasons", state.canSubmit)
    }

    @Test
    fun `KR41-04 submitting true with Other and text filled - canSubmit is false`() {
        val state = KickDialogState(
            selected = KickReason.Other,
            customText = "valid reason",
            submitting = true
        )
        assertFalse("submitting=true should prevent re-submission even with valid customText", state.canSubmit)
    }

    @Test
    fun `KR41-04 submitting false after completion - canSubmit restores to true`() {
        val submittingState = KickDialogState(selected = KickReason.Spam, submitting = true)
        assertFalse(submittingState.canSubmit)

        val doneState = submittingState.copy(submitting = false)
        assertTrue("After submitting completes, canSubmit should restore", doneState.canSubmit)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // KR41-07: KickReason key 正确性验证
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `KR41-07 KickReason Harassment key is harassment`() {
        assertEquals("harassment", KickReason.Harassment.key)
    }

    @Test
    fun `KR41-07 KickReason Spam key is spam`() {
        assertEquals("spam", KickReason.Spam.key)
    }

    @Test
    fun `KR41-07 KickReason Abuse key is abuse`() {
        assertEquals("abuse", KickReason.Abuse.key)
    }

    @Test
    fun `KR41-07 KickReason Other key is other`() {
        assertEquals("other", KickReason.Other.key)
    }

    @Test
    fun `KR41-07 preset reason canSubmit for all non-Other KickReasons`() {
        listOf(KickReason.Harassment, KickReason.Spam, KickReason.Abuse).forEach { reason ->
            val state = KickDialogState(selected = reason)
            assertTrue(
                "Preset reason $reason should always be submittable (no customText needed)",
                state.canSubmit
            )
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 边界情况：customText max 100 字符边界
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `KR41-07 Other with exactly 100 char customText - canSubmit is true`() {
        val text = "a".repeat(100)
        val state = KickDialogState(selected = KickReason.Other, customText = text)
        assertTrue("100-char customText should be valid", state.canSubmit)
    }

    @Test
    fun `KR41-07 KickReason has exactly 4 values`() {
        assertEquals("Should have exactly 4 KickReasons", 4, KickReason.values().size)
    }
}
