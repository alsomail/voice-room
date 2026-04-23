package com.voice.room.android.feature.room.governance

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * TDD 单元测试 — ActionMatrix 权限矩阵 (T-30040)
 *
 * 覆盖 UA40-01 ~ UA40-06 和 UA40-10：
 *   - 9 种角色组合（owner/admin/member × owner/admin/member）
 *   - ×2 目标在麦/不在麦
 *   - ×2 isSelf=true/false
 *
 * 测试策略：纯函数 [computeActions] 的黑盒测试，只验证输出行为。
 */
class ActionMatrixTest {

    // ──────────────────────────────────────────────────────────────────────────
    // UA40-01: member 看 member（他人）→ 仅 [ViewProfile, Report]
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `UA40-01 member sees other member - only ViewProfile and Report`() {
        val actions = computeActions(
            myRole = Role.Member,
            targetRole = Role.Member,
            targetOnMic = false,
            isSelf = false,
        )
        assertEquals(listOf(UserAction.ViewProfile, UserAction.Report), actions)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // UA40-02: admin 看 owner → 仅 [ViewProfile, Report]
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `UA40-02 admin sees owner - only ViewProfile and Report`() {
        val actions = computeActions(
            myRole = Role.Admin,
            targetRole = Role.Owner,
            targetOnMic = false,
            isSelf = false,
        )
        assertEquals(listOf(UserAction.ViewProfile, UserAction.Report), actions)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // UA40-03: owner 看 admin → 含 RevokeAdmin，不含 AssignAdmin
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `UA40-03 owner sees admin - contains RevokeAdmin not AssignAdmin`() {
        val actions = computeActions(
            myRole = Role.Owner,
            targetRole = Role.Admin,
            targetOnMic = false,
            isSelf = false,
        )
        assertTrue("RevokeAdmin should be present", UserAction.RevokeAdmin in actions)
        assertFalse("AssignAdmin should not be present", UserAction.AssignAdmin in actions)
        // 其他必须操作也应存在
        assertTrue(UserAction.MuteMic in actions)
        assertTrue(UserAction.MuteChat in actions)
        assertTrue(UserAction.Kick in actions)
        assertTrue(UserAction.ViewProfile in actions)
        assertTrue(UserAction.Report in actions)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // UA40-04: owner 看 member → 含 AssignAdmin，不含 RevokeAdmin
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `UA40-04 owner sees member - contains AssignAdmin not RevokeAdmin`() {
        val actions = computeActions(
            myRole = Role.Owner,
            targetRole = Role.Member,
            targetOnMic = false,
            isSelf = false,
        )
        assertTrue("AssignAdmin should be present", UserAction.AssignAdmin in actions)
        assertFalse("RevokeAdmin should not be present", UserAction.RevokeAdmin in actions)
        assertTrue(UserAction.Kick in actions)
        assertTrue(UserAction.ViewProfile in actions)
        assertTrue(UserAction.Report in actions)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // UA40-05: 目标在麦时只显示 ForceLeaveMic，不显示 ForceTakeMic
    //          目标不在麦时只显示 ForceTakeMic，不显示 ForceLeaveMic
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `UA40-05a target on mic - only ForceLeaveMic shown not ForceTakeMic`() {
        val actions = computeActions(
            myRole = Role.Owner,
            targetRole = Role.Member,
            targetOnMic = true,
            isSelf = false,
        )
        assertTrue("ForceLeaveMic should be present when target on mic", UserAction.ForceLeaveMic in actions)
        assertFalse("ForceTakeMic should not be present when target on mic", UserAction.ForceTakeMic in actions)
    }

    @Test
    fun `UA40-05b target off mic - only ForceTakeMic shown not ForceLeaveMic`() {
        val actions = computeActions(
            myRole = Role.Owner,
            targetRole = Role.Member,
            targetOnMic = false,
            isSelf = false,
        )
        assertTrue("ForceTakeMic should be present when target not on mic", UserAction.ForceTakeMic in actions)
        assertFalse("ForceLeaveMic should not be present when target not on mic", UserAction.ForceLeaveMic in actions)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // UA40-06: 自己看自己（isSelf=true）→ 无操作项
    // ──────────────────────────────────────────────────────────────────────────

    @Test
    fun `UA40-06 self view - empty action list`() {
        // owner 看自己
        assertTrue(
            "owner self → empty",
            computeActions(Role.Owner, Role.Owner, false, isSelf = true).isEmpty()
        )
        // admin 看自己
        assertTrue(
            "admin self → empty",
            computeActions(Role.Admin, Role.Admin, false, isSelf = true).isEmpty()
        )
        // member 看自己
        assertTrue(
            "member self → empty",
            computeActions(Role.Member, Role.Member, false, isSelf = true).isEmpty()
        )
    }

    // ──────────────────────────────────────────────────────────────────────────
    // UA40-10: 9 角色组合 × (在/不在麦) × (自己/他人) 全部覆盖
    // ──────────────────────────────────────────────────────────────────────────

    // --- owner 作为操作者 ---

    @Test
    fun `UA40-10 owner-owner other not on mic - ViewProfile and Report only`() {
        val actions = computeActions(Role.Owner, Role.Owner, false, isSelf = false)
        assertEquals(listOf(UserAction.ViewProfile, UserAction.Report), actions)
    }

    @Test
    fun `UA40-10 owner-owner self - empty`() {
        assertTrue(computeActions(Role.Owner, Role.Owner, false, isSelf = true).isEmpty())
    }

    @Test
    fun `UA40-10 owner-admin not on mic - RevokeAdmin ForceTakeMic MuteMic MuteChat Kick ViewProfile Report`() {
        val actions = computeActions(Role.Owner, Role.Admin, targetOnMic = false, isSelf = false)
        val expected = listOf(
            UserAction.RevokeAdmin,
            UserAction.ForceTakeMic,
            UserAction.MuteMic,
            UserAction.MuteChat,
            UserAction.Kick,
            UserAction.ViewProfile,
            UserAction.Report,
        )
        assertEquals(expected, actions)
    }

    @Test
    fun `UA40-10 owner-admin on mic - RevokeAdmin ForceLeaveMic MuteMic MuteChat Kick ViewProfile Report`() {
        val actions = computeActions(Role.Owner, Role.Admin, targetOnMic = true, isSelf = false)
        val expected = listOf(
            UserAction.RevokeAdmin,
            UserAction.ForceLeaveMic,
            UserAction.MuteMic,
            UserAction.MuteChat,
            UserAction.Kick,
            UserAction.ViewProfile,
            UserAction.Report,
        )
        assertEquals(expected, actions)
    }

    @Test
    fun `UA40-10 owner-member not on mic - AssignAdmin ForceTakeMic MuteMic MuteChat Kick ViewProfile Report`() {
        val actions = computeActions(Role.Owner, Role.Member, targetOnMic = false, isSelf = false)
        val expected = listOf(
            UserAction.AssignAdmin,
            UserAction.ForceTakeMic,
            UserAction.MuteMic,
            UserAction.MuteChat,
            UserAction.Kick,
            UserAction.ViewProfile,
            UserAction.Report,
        )
        assertEquals(expected, actions)
    }

    @Test
    fun `UA40-10 owner-member on mic - AssignAdmin ForceLeaveMic MuteMic MuteChat Kick ViewProfile Report`() {
        val actions = computeActions(Role.Owner, Role.Member, targetOnMic = true, isSelf = false)
        val expected = listOf(
            UserAction.AssignAdmin,
            UserAction.ForceLeaveMic,
            UserAction.MuteMic,
            UserAction.MuteChat,
            UserAction.Kick,
            UserAction.ViewProfile,
            UserAction.Report,
        )
        assertEquals(expected, actions)
    }

    // --- admin 作为操作者 ---

    @Test
    fun `UA40-10 admin-owner not on mic - ViewProfile Report only`() {
        val actions = computeActions(Role.Admin, Role.Owner, false, isSelf = false)
        assertEquals(listOf(UserAction.ViewProfile, UserAction.Report), actions)
    }

    @Test
    fun `UA40-10 admin-admin other not on mic - ViewProfile Report only`() {
        val actions = computeActions(Role.Admin, Role.Admin, false, isSelf = false)
        assertEquals(listOf(UserAction.ViewProfile, UserAction.Report), actions)
    }

    @Test
    fun `UA40-10 admin-admin self - empty`() {
        assertTrue(computeActions(Role.Admin, Role.Admin, false, isSelf = true).isEmpty())
    }

    @Test
    fun `UA40-10 admin-member not on mic - ForceTakeMic MuteMic MuteChat Kick ViewProfile Report`() {
        val actions = computeActions(Role.Admin, Role.Member, targetOnMic = false, isSelf = false)
        val expected = listOf(
            UserAction.ForceTakeMic,
            UserAction.MuteMic,
            UserAction.MuteChat,
            UserAction.Kick,
            UserAction.ViewProfile,
            UserAction.Report,
        )
        assertEquals(expected, actions)
    }

    @Test
    fun `UA40-10 admin-member on mic - ForceLeaveMic MuteMic MuteChat Kick ViewProfile Report`() {
        val actions = computeActions(Role.Admin, Role.Member, targetOnMic = true, isSelf = false)
        val expected = listOf(
            UserAction.ForceLeaveMic,
            UserAction.MuteMic,
            UserAction.MuteChat,
            UserAction.Kick,
            UserAction.ViewProfile,
            UserAction.Report,
        )
        assertEquals(expected, actions)
    }

    // --- member 作为操作者 ---

    @Test
    fun `UA40-10 member-owner not on mic - ViewProfile Report only`() {
        val actions = computeActions(Role.Member, Role.Owner, false, isSelf = false)
        assertEquals(listOf(UserAction.ViewProfile, UserAction.Report), actions)
    }

    @Test
    fun `UA40-10 member-admin not on mic - ViewProfile Report only`() {
        val actions = computeActions(Role.Member, Role.Admin, false, isSelf = false)
        assertEquals(listOf(UserAction.ViewProfile, UserAction.Report), actions)
    }

    @Test
    fun `UA40-10 member-member other not on mic - ViewProfile Report only`() {
        val actions = computeActions(Role.Member, Role.Member, false, isSelf = false)
        assertEquals(listOf(UserAction.ViewProfile, UserAction.Report), actions)
    }

    @Test
    fun `UA40-10 member-member self - empty`() {
        assertTrue(computeActions(Role.Member, Role.Member, false, isSelf = true).isEmpty())
    }

    // --- onMic 对 ViewProfile/Report-only 组合无影响 ---

    @Test
    fun `UA40-10 targetOnMic does not affect member-member actions`() {
        val offMic = computeActions(Role.Member, Role.Member, targetOnMic = false, isSelf = false)
        val onMic = computeActions(Role.Member, Role.Member, targetOnMic = true, isSelf = false)
        assertEquals("on-mic and off-mic should be same for member-member", offMic, onMic)
    }

    @Test
    fun `UA40-10 targetOnMic does not affect admin-owner actions`() {
        val offMic = computeActions(Role.Admin, Role.Owner, targetOnMic = false, isSelf = false)
        val onMic = computeActions(Role.Admin, Role.Owner, targetOnMic = true, isSelf = false)
        assertEquals(offMic, onMic)
    }

    // --- 边界：isSelf 优先于角色判断 ---

    @Test
    fun `UA40-10 isSelf true overrides all role combinations`() {
        data class Case(val my: Role, val target: Role, val onMic: Boolean)
        val combinations = listOf(
            Case(Role.Owner, Role.Admin, false),
            Case(Role.Owner, Role.Member, true),
            Case(Role.Admin, Role.Member, false),
            Case(Role.Admin, Role.Member, true),
            Case(Role.Member, Role.Owner, false),
        )
        combinations.forEach { (my, target, onMic) ->
            val actions = computeActions(my, target, onMic, isSelf = true)
            assertTrue(
                "isSelf=true should produce empty actions for $my vs $target onMic=$onMic",
                actions.isEmpty()
            )
        }
    }
}
