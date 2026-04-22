package com.voice.room.android.feature.room

import com.voice.room.android.core.media.FakeMediaService
import com.voice.room.android.core.ws.FakeWebSocketClient
import com.voice.room.android.data.model.RoomMember
import com.voice.room.android.data.room.FakeRoomMemberRepository
import com.voice.room.android.data.room.IRoomMemberRepository
import com.voice.room.android.data.room.MemberListResult
import com.voice.room.android.data.room.MicSlotData
import com.voice.room.android.data.room.RoomSnapshot
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test

/**
 * TDD 单元测试 — 观众席 BottomSheet ViewModel (T-30039)
 *
 * A39-01: 空记录时双空状态 — onMic 和 audience 均为空列表
 * A39-02: 麦上用户始终置顶 — onMic 区独立于 audience 区
 * A39-03: WS UserJoined → audience 新增该用户
 * A39-04: WS MicTaken → 用户从 audience 移到 onMic
 * A39-05: hasMore=true 时 loadMoreMembers 增加 currentPage
 * A39-06: onMemberClick 触发 selectedMember 更新
 * A39-07: role='owner' 的 member 保留 owner 角色标记
 */
@OptIn(ExperimentalCoroutinesApi::class)
class AudienceViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var fakeWsClient: FakeWebSocketClient
    private lateinit var fakeSnapshotRepo: FakeRoomSnapshotRepository
    private lateinit var fakeMediaService: FakeMediaService
    private lateinit var fakeMemberRepo: FakeRoomMemberRepository
    private lateinit var viewModel: RoomViewModel

    private val emptySnapshot = RoomSnapshot(
        roomId = "room-1",
        roomName = "Test Room",
        onlineCount = 0,
        micSlots = emptyList()
    )

    @Before
    fun setup() {
        fakeWsClient = FakeWebSocketClient()
        fakeSnapshotRepo = FakeRoomSnapshotRepository(emptySnapshot)
        fakeMediaService = FakeMediaService()
        fakeMemberRepo = FakeRoomMemberRepository()
        viewModel = RoomViewModel(
            wsClient = fakeWsClient,
            roomSnapshotRepository = fakeSnapshotRepo,
            mediaService = fakeMediaService,
            memberRepository = fakeMemberRepo,
        )
    }

    // ─── A39-01: 初始状态双空 ────────────────────────────────────────────────────

    @Test
    fun `A39-01 initial audienceState - onMic and audience are both empty`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val state = viewModel.audienceState.value

            assertTrue("onMic should be empty on init", state.onMic.isEmpty())
            assertTrue("audience should be empty on init", state.audience.isEmpty())
            assertEquals("total should be 0 on init", 0, state.total)
        }

    // ─── A39-02: onMic 用户始终置顶 ────────────────────────────────────────────

    @Test
    fun `A39-02 after MicTaken user is in onMic section not audience`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // 用户先加入观众
            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","userId":"u1","nickname":"Nick1","role":"member"}"""
            )
            advanceUntilIdle()

            // 用户上麦
            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","slotIndex":0,"userId":"u1","nickname":"Nick1"}"""
            )
            advanceUntilIdle()

            val state = viewModel.audienceState.value
            assertTrue(
                "onMic should contain u1 after MicTaken",
                state.onMic.any { it.id == "u1" }
            )
            assertFalse(
                "audience should NOT contain u1 after MicTaken",
                state.audience.any { it.id == "u1" }
            )
        }

    // ─── A39-03: WS UserJoined → audience 新增该用户 ───────────────────────────

    @Test
    fun `A39-03 WS UserJoined - user added to audience tail`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","userId":"u1","nickname":"Nick1","role":"member"}"""
            )
            advanceUntilIdle()

            val state = viewModel.audienceState.value
            assertTrue(
                "audience should contain u1 after UserJoined",
                state.audience.any { it.id == "u1" }
            )
            assertEquals(
                "audience should have exactly 1 member",
                1,
                state.audience.size
            )
        }

    // ─── A39-04: WS MicTaken → 用户从 audience 移到 onMic ─────────────────────

    @Test
    fun `A39-04 WS MicTaken - user moves from audience to onMic`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // 先加入观众席
            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","userId":"u2","nickname":"Nick2","role":"member"}"""
            )
            advanceUntilIdle()

            // 确认在观众席
            assertTrue(
                "u2 should be in audience before MicTaken",
                viewModel.audienceState.value.audience.any { it.id == "u2" }
            )

            // 上麦
            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","slotIndex":1,"userId":"u2","nickname":"Nick2"}"""
            )
            advanceUntilIdle()

            val state = viewModel.audienceState.value
            assertTrue(
                "onMic should contain u2 after MicTaken",
                state.onMic.any { it.id == "u2" }
            )
            assertFalse(
                "audience should NOT contain u2 after MicTaken",
                state.audience.any { it.id == "u2" }
            )
            // 验证 slot 被记录
            val onMicUser = state.onMic.find { it.id == "u2" }
            assertEquals("slot should be 1", 1, onMicUser?.slot)
        }

    // ─── A39-05: hasMore=true 时 loadMoreMembers 增加 currentPage ───────────────

    @Test
    fun `A39-05 loadMoreMembers when hasMore true - increments currentPage`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            val initialPage = viewModel.audienceState.value.currentPage
            assertTrue(
                "hasMore should be true initially",
                viewModel.audienceState.value.hasMore
            )

            viewModel.loadMoreMembers()
            advanceUntilIdle()

            val newPage = viewModel.audienceState.value.currentPage
            assertEquals(
                "currentPage should increment by 1",
                initialPage + 1,
                newPage
            )
        }

    // ─── A39-06: onMemberClick → selectedMember 更新 ───────────────────────────

    @Test
    fun `A39-06 onMemberClick - selectedMember is updated to clicked member`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val member = RoomMember(id = "u3", nickname = "Nick3", role = "member")

            viewModel.onMemberClick(member)
            advanceUntilIdle()

            assertEquals(
                "selectedMember should be the clicked member",
                member,
                viewModel.selectedMember.value
            )
        }

    // ─── A39-07: role='owner' 的 member 保留 owner 标记 ─────────────────────────

    @Test
    fun `A39-07 UserJoined with role owner - member retains owner role in audience`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","userId":"owner1","nickname":"RoomOwner","role":"owner"}"""
            )
            advanceUntilIdle()

            val state = viewModel.audienceState.value
            val owner = state.audience.find { it.id == "owner1" }

            assertNotNull("Owner user should be in audience", owner)
            assertEquals(
                "role should be 'owner'",
                "owner",
                owner?.role
            )
        }

    // ─── A39 额外: WS UserLeft → 从 audience 移除 ──────────────────────────────

    @Test
    fun `A39-extra-1 WS UserLeft - user removed from audience`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","userId":"u4","nickname":"Nick4","role":"member"}"""
            )
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"UserLeft","userId":"u4"}"""
            )
            advanceUntilIdle()

            val state = viewModel.audienceState.value
            assertFalse(
                "audience should NOT contain u4 after UserLeft",
                state.audience.any { it.id == "u4" }
            )
        }

    // ─── A39 额外: WS MicLeft → 从 onMic 移回 audience ─────────────────────────

    @Test
    fun `A39-extra-2 WS MicLeft - user moves from onMic back to audience`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // 先加入 → 上麦
            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","userId":"u5","nickname":"Nick5","role":"member"}"""
            )
            advanceUntilIdle()
            fakeWsClient.simulateMessage(
                """{"type":"MicTaken","slotIndex":2,"userId":"u5","nickname":"Nick5"}"""
            )
            advanceUntilIdle()

            // 下麦
            fakeWsClient.simulateMessage(
                """{"type":"MicLeft","slotIndex":2,"userId":"u5"}"""
            )
            advanceUntilIdle()

            val state = viewModel.audienceState.value
            assertFalse(
                "onMic should NOT contain u5 after MicLeft",
                state.onMic.any { it.id == "u5" }
            )
            assertTrue(
                "audience should contain u5 after MicLeft",
                state.audience.any { it.id == "u5" }
            )
        }

    // ─── A39 额外: WS AdminChanged → 更新 role ──────────────────────────────────

    @Test
    fun `A39-extra-3 WS AdminChanged - updates member role in audience`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"UserJoined","userId":"u6","nickname":"Nick6","role":"member"}"""
            )
            advanceUntilIdle()

            fakeWsClient.simulateMessage(
                """{"type":"AdminChanged","userId":"u6","role":"admin"}"""
            )
            advanceUntilIdle()

            val state = viewModel.audienceState.value
            val member = state.audience.find { it.id == "u6" }
            assertEquals("role should be updated to admin", "admin", member?.role)
        }

    // ─── A39 额外: loadMoreMembers 加载结果追加到 audience ──────────────────────

    @Test
    fun `A39-extra-4 loadMoreMembers appends fetched members to audience`() =
        runTest(mainDispatcherRule.testDispatcher) {
            fakeMemberRepo.result = MemberListResult(
                members = listOf(
                    RoomMember(id = "api-u1", nickname = "ApiUser1", role = "member"),
                    RoomMember(id = "api-u2", nickname = "ApiUser2", role = "member"),
                ),
                total = 2,
                hasMore = false
            )

            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            viewModel.loadMoreMembers()
            advanceUntilIdle()

            val state = viewModel.audienceState.value
            assertTrue(
                "audience should contain api-u1",
                state.audience.any { it.id == "api-u1" }
            )
            assertTrue(
                "audience should contain api-u2",
                state.audience.any { it.id == "api-u2" }
            )
            assertFalse(
                "hasMore should be false after exhausted pages",
                state.hasMore
            )
        }

    // ─── A39 额外: loadMoreMembers when hasMore=false → 不增加 currentPage ──────

    @Test
    fun `A39-extra-5 loadMoreMembers when hasMore false - does NOT increment currentPage`() =
        runTest(mainDispatcherRule.testDispatcher) {
            viewModel.joinRoom("room-1")
            advanceUntilIdle()

            // 先消耗页数到 hasMore = false
            fakeMemberRepo.result = MemberListResult(
                members = emptyList(), total = 0, hasMore = false
            )
            viewModel.loadMoreMembers()
            advanceUntilIdle()

            val pageAfterFirst = viewModel.audienceState.value.currentPage

            // 再次调用，hasMore 已为 false，不应再增加
            viewModel.loadMoreMembers()
            advanceUntilIdle()

            assertEquals(
                "currentPage should NOT change when hasMore is false",
                pageAfterFirst,
                viewModel.audienceState.value.currentPage
            )
        }
}
