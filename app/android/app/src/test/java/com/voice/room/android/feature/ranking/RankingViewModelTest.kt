package com.voice.room.android.feature.ranking

import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.domain.ranking.IRankingRepository
import com.voice.room.android.domain.ranking.MyRank
import com.voice.room.android.domain.ranking.RankEntry
import com.voice.room.android.domain.ranking.RankingPage
import com.voice.room.android.utils.MainDispatcherRule
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import java.io.IOException

/**
 * TDD 单元测试 — RankingViewModel (T-30033)
 *
 * R33-01: 打开默认加载魅力-日榜
 * R33-02: 切换到周榜重新调 API
 * R33-03: Top3 头像带对应光圈，Top1 显示王冠
 * R33-04: 未入榜时 MyRankFooter 显示"未上榜，继续加油"
 * R33-05: 下拉刷新触发 API 重试
 * R33-06: 网络错误显示重试按钮
 * R33-07: 切换财富榜重新调 API
 * R33-08: 401 错误发射 NavigateToLogin 事件
 */
@OptIn(ExperimentalCoroutinesApi::class)
class RankingViewModelTest {

    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    // ─── Fake Repository ──────────────────────────────────────────────────────

    private class FakeRankingRepository(
        var result: Result<RankingPage> = Result.success(defaultPage()),
        private val throwCancellation: Boolean = false,
    ) : IRankingRepository {
        var lastType: String? = null
        var lastPeriod: String? = null
        var callCount: Int = 0

        override suspend fun getRanking(type: String, period: String): Result<RankingPage> {
            callCount++
            lastType = type
            lastPeriod = period
            if (throwCancellation) throw CancellationException("cancelled")
            return result
        }
    }

    // ─── Helpers ──────────────────────────────────────────────────────────────

    private fun buildViewModel(
        repo: IRankingRepository = FakeRankingRepository(),
    ): RankingViewModel = RankingViewModel(repo)

    // ─── R33-01: 打开默认加载魅力-日榜 ──────────────────────────────────────

    @Test
    fun `R33-01 default state loads charm daily ranking on init`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRankingRepository()
            val vm = buildViewModel(fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            assertEquals("charm", fakeRepo.lastType)
            assertEquals("day", fakeRepo.lastPeriod)
            assertFalse(vm.uiState.value.loading)
            assertEquals(3, vm.uiState.value.items.size)
        }

    // ─── R33-02: 切换到周榜重新调 API ─────────────────────────────────────────

    @Test
    fun `R33-02 switching to weekly period triggers new API call`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRankingRepository()
            val vm = buildViewModel(fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()

            val callsAfterInit = fakeRepo.callCount

            vm.selectPeriod(Period.Week)
            advanceUntilIdle()
            job.cancel()

            assertTrue(fakeRepo.callCount > callsAfterInit)
            assertEquals("week", fakeRepo.lastPeriod)
        }

    // ─── R33-03: Top3 头像带对应光圈颜色 ─────────────────────────────────────

    @Test
    fun `R33-03 top 3 items have correct medal values`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel(FakeRankingRepository())
            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            val items = vm.uiState.value.items
            assertTrue("At least 3 items", items.size >= 3)
            assertEquals("gold", items[0].medal)
            assertEquals("silver", items[1].medal)
            assertEquals("bronze", items[2].medal)
            assertEquals(1, items[0].rank)
        }

    // ─── R33-04: 未入榜时 myRank.rank==null ──────────────────────────────────

    @Test
    fun `R33-04 when user not in ranking myRank rank is null`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val pageWithNoMe = defaultPage().copy(me = MyRank(rank = null, score = 0))
            val fakeRepo = FakeRankingRepository(result = Result.success(pageWithNoMe))
            val vm = buildViewModel(fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            assertNull("myRank.rank should be null when not ranked", vm.uiState.value.myRank?.rank)
        }

    // ─── R33-05: 下拉刷新触发 API 重试 ────────────────────────────────────────

    @Test
    fun `R33-05 pull to refresh triggers another API call`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRankingRepository()
            val vm = buildViewModel(fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            val callsAfterInit = fakeRepo.callCount

            vm.refresh()
            advanceUntilIdle()
            job.cancel()

            assertTrue("refresh should call API again", fakeRepo.callCount > callsAfterInit)
        }

    // ─── R33-06: 网络错误 → error 非 null ────────────────────────────────────

    @Test
    fun `R33-06 network error sets error in uiState`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRankingRepository(
                result = Result.failure(IOException("Network unavailable"))
            )
            val vm = buildViewModel(fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            assertNotNull("error should be set on network failure", vm.uiState.value.error)
            assertFalse(vm.uiState.value.loading)
        }

    // ─── R33-07: 切换财富榜重新调 API ─────────────────────────────────────────

    @Test
    fun `R33-07 switching to wealth type triggers new API call with wealth type`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRankingRepository()
            val vm = buildViewModel(fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()

            val callsAfterInit = fakeRepo.callCount

            vm.selectType(RankingType.Wealth)
            advanceUntilIdle()
            job.cancel()

            assertTrue("selectType(Wealth) should trigger new API call", fakeRepo.callCount > callsAfterInit)
            assertEquals("wealth", fakeRepo.lastType)
        }

    // ─── R33-08: 401 → NavigateToLogin 事件 ──────────────────────────────────

    @Test
    fun `R33-08 401 ApiException emits NavigateToLogin event`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRankingRepository(
                result = Result.failure(ApiException(401, "Unauthorized"))
            )
            val vm = buildViewModel(fakeRepo)

            val events = mutableListOf<RankingEvent>()
            val eventsJob = launch(UnconfinedTestDispatcher(testScheduler)) {
                vm.events.collect { events.add(it) }
            }

            advanceUntilIdle()
            eventsJob.cancel()

            assertTrue(
                "Should emit NavigateToLogin on 401, got: $events",
                events.contains(RankingEvent.NavigateToLogin)
            )
        }

    // ─── Extra-01: CancellationException 被正确 re-throw ────────────────────

    @Test
    fun `Extra-01 CancellationException does not set error state`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val fakeRepo = FakeRankingRepository(throwCancellation = true)
            val vm = buildViewModel(fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            assertNull("CancellationException must not set error", vm.uiState.value.error)
        }

    // ─── Extra-02: 空榜单（items 为空）不崩溃 ────────────────────────────────

    @Test
    fun `Extra-02 empty ranking list does not crash`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val emptyPage = RankingPage(
                type = "charm", period = "day",
                items = emptyList(),
                me = MyRank(rank = null, score = 0)
            )
            val fakeRepo = FakeRankingRepository(result = Result.success(emptyPage))
            val vm = buildViewModel(fakeRepo)

            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            job.cancel()

            assertTrue("items should be empty", vm.uiState.value.items.isEmpty())
            assertFalse(vm.uiState.value.loading)
        }

    // ─── Extra-03: 刷新后 refreshing 恢复为 false ────────────────────────────

    @Test
    fun `Extra-03 refreshing becomes false after refresh completes`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val vm = buildViewModel(FakeRankingRepository())
            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()

            vm.refresh()
            advanceUntilIdle()
            job.cancel()

            assertFalse("refreshing must be false after refresh", vm.uiState.value.refreshing)
        }

    // ─── Extra-04: 切换 Tab 时清除旧错误 ─────────────────────────────────────

    @Test
    fun `Extra-04 selectPeriod clears previous error state`() =
        runTest(mainDispatcherRule.testDispatcher) {
            var callCount = 0
            val repo = object : IRankingRepository {
                override suspend fun getRanking(type: String, period: String): Result<RankingPage> {
                    callCount++
                    return if (callCount == 1) Result.failure(IOException("fail"))
                    else Result.success(defaultPage())
                }
            }
            val vm = buildViewModel(repo)
            val job = launch(UnconfinedTestDispatcher(testScheduler)) { vm.uiState.collect {} }
            advanceUntilIdle()
            assertNotNull(vm.uiState.value.error)

            vm.selectPeriod(Period.Week)
            advanceUntilIdle()
            job.cancel()

            assertNull("Switching period should clear error", vm.uiState.value.error)
        }

    // ─── HIGH-01: 竞态条件 — Tab 切换必须取消旧协程 ─────────────────────────

    /**
     * HIGH-01-a: 快速切换 Tab 时，前一个加载协程被取消，不会覆盖新 Tab 数据。
     *
     * 模拟：init 启动第一个 loadRanking（慢，10 秒延迟），
     *       此时立即调用 selectType(Wealth)（应取消第一个并启动第二个），
     *       最终 items 应来自第二个调用（Wealth）而不是第一个（Charm）。
     */
    @Test
    fun `HIGH-01-a rapid selectType cancels previous loading job`() =
        runTest(mainDispatcherRule.testDispatcher) {
            var firstCallCompleted = false
            var callCount = 0

            val repo = object : IRankingRepository {
                override suspend fun getRanking(type: String, period: String): Result<RankingPage> {
                    callCount++
                    return if (callCount == 1) {
                        // First call (charm/day from init) is very slow
                        delay(10_000L)
                        firstCallCompleted = true
                        Result.success(defaultPage().copy(type = "charm"))
                    } else {
                        // Second call (wealth) returns immediately
                        Result.success(
                            defaultPage().copy(
                                type = "wealth",
                                items = listOf(
                                    RankEntry(rank = 1, userId = "w1", nickname = "Wealthy", avatar = "", score = 99999, medal = "gold"),
                                )
                            )
                        )
                    }
                }
            }

            val vm = RankingViewModel(repo)
            // Start init coroutine but don't complete it (it's stuck at delay(10_000L))
            runCurrent()

            // While init is in-flight, switch type → must cancel init's job
            vm.selectType(RankingType.Wealth)
            advanceUntilIdle()

            // The first call (Charm) was cancelled before completing
            assertFalse(
                "First loadRanking job must have been cancelled, not allowed to complete",
                firstCallCompleted
            )
            // Final state reflects Wealth (second call), not Charm
            assertEquals(
                "UI should reflect Wealth type after selectType",
                RankingType.Wealth,
                vm.uiState.value.type
            )
        }

    /**
     * HIGH-01-b: 快速切换 Period 后再切回时，中间的慢请求被取消，
     * 不会用"旧 Tab"数据覆盖"新 Tab"的 UI。
     *
     * 关键断言：Week 调用耗时 10 秒，Day 调用即时返回；若没有取消，
     * Week 会在 Day 之后写入 state，导致 items 内容来自 Week。
     * 修复后 Week 被取消，最终 items 来自 Day 调用。
     */
    @Test
    fun `HIGH-01-b in-flight week request cancelled when switching back to day`() =
        runTest(mainDispatcherRule.testDispatcher) {
            val dayItems = listOf(
                RankEntry(rank = 1, userId = "d1", nickname = "DayUser", avatar = "", score = 111, medal = "gold")
            )
            val weekItems = listOf(
                RankEntry(rank = 1, userId = "w1", nickname = "WeekUser", avatar = "", score = 999, medal = "gold")
            )
            var callCount = 0

            val repo = object : IRankingRepository {
                override suspend fun getRanking(type: String, period: String): Result<RankingPage> {
                    callCount++
                    return when (callCount) {
                        1 -> Result.success(defaultPage().copy(items = dayItems, period = "day"))
                        2 -> {
                            // Week request — very slow, should get cancelled
                            delay(10_000L)
                            Result.success(defaultPage().copy(items = weekItems, period = "week"))
                        }
                        else -> {
                            // Day again after switching back — immediate
                            Result.success(defaultPage().copy(items = dayItems, period = "day"))
                        }
                    }
                }
            }

            val vm = RankingViewModel(repo)
            advanceUntilIdle() // init (Day) completes, items = dayItems

            // Switch to Week (slow)
            vm.selectPeriod(Period.Week)
            runCurrent() // Week coroutine starts but stalls at delay(10_000)

            // Before Week completes, switch back to Day — must CANCEL Week coroutine
            vm.selectPeriod(Period.Day)
            advanceUntilIdle()

            // Without fix: Week completes after Day → items == weekItems (WRONG)
            // With fix:    Week is cancelled     → items == dayItems  (CORRECT)
            assertEquals(
                "Week request must be cancelled; items should come from Day call",
                "DayUser",
                vm.uiState.value.items.firstOrNull()?.nickname
            )
        }

    companion object {
        fun defaultPage() = RankingPage(
            type = "charm",
            period = "day",
            items = listOf(
                RankEntry(rank = 1, userId = "u1", nickname = "Alice", avatar = "", score = 10000, medal = "gold"),
                RankEntry(rank = 2, userId = "u2", nickname = "Bob",   avatar = "", score = 8000,  medal = "silver"),
                RankEntry(rank = 3, userId = "u3", nickname = "Carol", avatar = "", score = 6000,  medal = "bronze"),
            ),
            me = MyRank(rank = 42, score = 500)
        )
    }
}
