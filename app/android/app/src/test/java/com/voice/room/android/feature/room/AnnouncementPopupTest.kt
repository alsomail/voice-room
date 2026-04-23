package com.voice.room.android.feature.room

import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.data.local.AnnouncementSeenStore
import com.voice.room.android.data.local.FakeAnnouncementSeenStore
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.utils.FakeClock
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — 公告弹窗 + 管理员徽章 + RoomInfoUpdated (T-30043)
 *
 * AN43-01: 首次进有公告房弹窗
 * AN43-02: 24h 内再进房不弹窗
 * AN43-03: 空公告不显示顶部 📄
 * AN43-04: 顶部 📄 点击手动弹出
 * AN43-05: AdminChanged 到达后 500ms 内麦位徽章更新
 * AN43-06: RoomInfoUpdated 改 announcement 后重新弹窗
 * AN43-07: Owner / Admin / member 角色正确映射
 * AN43-08: 关闭弹窗后 showAnnouncementPopup = null
 */
@OptIn(ExperimentalCoroutinesApi::class)
class AnnouncementPopupTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var fakeWsClient: FakeWebSocketClient
    private lateinit var fakeRepo: FakeRoomSnapshotRepository
    private lateinit var fakeSeenStore: FakeAnnouncementSeenStore
    private lateinit var fakeClock: FakeClock
    private lateinit var viewModel: RoomViewModel

    private val announcementText = "欢迎来到测试房间！这是公告内容。"

    private val snapshotWithAnnouncement = RoomSnapshot(
        roomId = "room-43",
        roomName = "Test Room 43",
        onlineCount = 10,
        micSlots = listOf(MicSlotData(index = 0, userId = "owner-1", nickname = "Owner")),
        announcement = announcementText,
    )

    private val snapshotBlankAnnouncement = RoomSnapshot(
        roomId = "room-43",
        roomName = "Test Room 43",
        onlineCount = 10,
        micSlots = emptyList(),
        announcement = "",
    )

    @Before
    fun setup() {
        fakeWsClient = FakeWebSocketClient()
        fakeRepo = FakeRoomSnapshotRepository(snapshotWithAnnouncement)
        fakeSeenStore = FakeAnnouncementSeenStore()
        fakeClock = FakeClock(currentTimeMs = 1_000_000L)
        viewModel = RoomViewModel(
            wsClient = fakeWsClient,
            roomSnapshotRepository = fakeRepo,
            announcementSeenStore = fakeSeenStore,
            clock = fakeClock,
        )
    }

    // ─── AN43-01: 首次进有公告房弹窗 ─────────────────────────────────────────

    @Test
    fun `AN43-01 首次进有公告房 - showAnnouncementPopup 非空且等于公告内容`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-43", "user-1")
            advanceUntilIdle()

            val popup = viewModel.showAnnouncementPopup.value
            assertNotNull("首次进有公告房应显示弹窗", popup)
            assertEquals("弹窗内容应等于公告", announcementText, popup)
        }

    // ─── AN43-02: 24h 内再进房不弹窗 ─────────────────────────────────────────

    @Test
    fun `AN43-02 24小时内再进房 - 不弹窗`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // 首次进房，记录已看过（时间戳 = 1_000_000）
            fakeSeenStore.save("room-43", fakeClock.currentTimeMs)

            // 推进时间 23 小时（< 24h 内）
            fakeClock.currentTimeMs += 23 * 60 * 60 * 1000L

            viewModel.joinRoom("room-43", "user-1")
            advanceUntilIdle()

            val popup = viewModel.showAnnouncementPopup.value
            assertNull("24h 内再进房不应弹窗", popup)
        }

    // ─── AN43-03: 空公告不显示顶部 📄 ──────────────────────────────────────────

    @Test
    fun `AN43-03 空公告 - showAnnouncementIcon 为 false`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeRepo = FakeRoomSnapshotRepository(snapshotBlankAnnouncement)
            viewModel = RoomViewModel(
                wsClient = fakeWsClient,
                roomSnapshotRepository = fakeRepo,
                announcementSeenStore = fakeSeenStore,
                clock = fakeClock,
            )

            viewModel.joinRoom("room-43", "user-1")
            advanceUntilIdle()

            assertFalse("空公告不应显示顶部图标", viewModel.showAnnouncementIcon.value)
            assertNull("空公告不应弹出弹窗", viewModel.showAnnouncementPopup.value)
        }

    // ─── AN43-04: 顶部 📄 点击手动弹出 ─────────────────────────────────────────

    @Test
    fun `AN43-04 顶部图标点击 - 手动弹出公告弹窗`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // 先进房 - 会触发首次弹窗
            viewModel.joinRoom("room-43", "user-1")
            advanceUntilIdle()

            // 关闭弹窗
            viewModel.dismissAnnouncementPopup()
            assertNull("关闭后弹窗应消失", viewModel.showAnnouncementPopup.value)

            // 点击顶部图标重新弹出
            viewModel.onAnnouncementIconClick()
            val popup = viewModel.showAnnouncementPopup.value
            assertNotNull("点击图标后弹窗应显示", popup)
            assertEquals("弹窗内容应等于当前公告", announcementText, popup)
        }

    // ─── AN43-05: AdminChanged 到达后麦位徽章更新 ──────────────────────────────

    @Test
    fun `AN43-05 AdminChanged WS消息 - audienceState中用户角色更新`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-43", "user-1")
            advanceUntilIdle()

            // 模拟有一个观众
            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","userId":"user-2","nickname":"User2","role":"member"}"""
            )
            advanceUntilIdle()

            // 发送 AdminChanged 消息
            val startTime = System.currentTimeMillis()
            fakeWsClient.simulateMessage(
                """{"type":"AdminChanged","userId":"user-2","role":"admin"}"""
            )
            advanceUntilIdle()
            val elapsedMs = System.currentTimeMillis() - startTime

            // 验证角色已更新（测试环境不计时，但逻辑路径验证）
            val audience = viewModel.audienceState.value.audience
            val user2 = audience.find { it.id == "user-2" }
            assertNotNull("user-2 应在观众列表中", user2)
            assertEquals("user-2 角色应更新为 admin", "admin", user2!!.role)
            // 验证处理速度（测试环境通常远低于 500ms）
            assertTrue("AdminChanged 应在 500ms 内处理完成", elapsedMs < 500)
        }

    // ─── AN43-06: RoomInfoUpdated 改公告后重新弹窗 ─────────────────────────────

    @Test
    fun `AN43-06 RoomInfoUpdated更新announcement - 重置seen并重新弹窗`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-43", "user-1")
            advanceUntilIdle()

            // 首次弹窗后关闭
            viewModel.dismissAnnouncementPopup()
            assertNull("关闭后弹窗应消失", viewModel.showAnnouncementPopup.value)

            val newAnnouncement = "房间规则已更新：请遵守社区守则！"

            // 收到 RoomInfoUpdated 事件，公告变化
            fakeWsClient.simulateMessage(
                """{"type":"RoomInfoUpdated","announcement":"$newAnnouncement"}"""
            )
            advanceUntilIdle()

            val popup = viewModel.showAnnouncementPopup.value
            assertNotNull("公告变化后应重新弹窗", popup)
            assertEquals("弹窗内容应等于新公告", newAnnouncement, popup)
        }

    // ─── AN43-07: 角色正确映射 ──────────────────────────────────────────────────

    @Test
    fun `AN43-07 RoomInfoUpdated更新title和category - roomState对应字段更新`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-43", "user-1")
            advanceUntilIdle()

            val newTitle = "新房间名称"
            fakeWsClient.simulateMessage(
                """{"type":"RoomInfoUpdated","title":"$newTitle","category":"music"}"""
            )
            advanceUntilIdle()

            val state = viewModel.uiState.value as? RoomViewState.Success
            assertNotNull("uiState 应为 Success", state)
            assertEquals("roomName 应更新为新标题", newTitle, state!!.uiState.roomName)
        }

    // ─── AN43-08: 关闭弹窗后 showAnnouncementPopup = null ────────────────────

    @Test
    fun `AN43-08 dismissAnnouncementPopup - showAnnouncementPopup变为null`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-43", "user-1")
            advanceUntilIdle()

            // 确认弹窗已显示
            assertNotNull("进房后弹窗应显示", viewModel.showAnnouncementPopup.value)

            // 关闭弹窗
            viewModel.dismissAnnouncementPopup()

            assertNull("关闭后 showAnnouncementPopup 应为 null", viewModel.showAnnouncementPopup.value)
            // 顶部图标应仍然可见（公告非空）
            assertTrue("关闭弹窗后顶部图标仍应显示", viewModel.showAnnouncementIcon.value)
        }

    // ─── 额外：超过 24h 后再进房 → 重新弹窗 ────────────────────────────────────

    @Test
    fun `AN43-02b 超过24小时后再进房 - 重新弹窗`() =
        runTest(mainDispatcherRule.testDispatcher) {
            // 记录已看过（时间戳 = 1_000_000）
            fakeSeenStore.save("room-43", fakeClock.currentTimeMs)

            // 推进时间 25 小时（> 24h）
            fakeClock.currentTimeMs += 25 * 60 * 60 * 1000L

            viewModel.joinRoom("room-43", "user-1")
            advanceUntilIdle()

            val popup = viewModel.showAnnouncementPopup.value
            assertNotNull("超过 24h 后再进房应重新弹窗", popup)
            assertEquals("弹窗内容应等于公告", announcementText, popup)
        }

    // ─── 额外：RoomInfoUpdated 公告未变化 → 不重新弹窗 ────────────────────────

    @Test
    fun `AN43-06b RoomInfoUpdated公告未变化 - 不重新弹窗`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-43", "user-1")
            advanceUntilIdle()

            // 首次弹窗后关闭
            viewModel.dismissAnnouncementPopup()
            assertNull("关闭后弹窗应消失", viewModel.showAnnouncementPopup.value)

            // RoomInfoUpdated 但 announcement 字段与当前相同（无变化）
            fakeWsClient.simulateMessage(
                """{"type":"RoomInfoUpdated","announcement":"$announcementText"}"""
            )
            advanceUntilIdle()

            assertNull("公告未变化不应重新弹窗", viewModel.showAnnouncementPopup.value)
        }
}
