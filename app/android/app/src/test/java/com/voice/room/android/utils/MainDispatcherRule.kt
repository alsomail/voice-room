package com.voice.room.android.utils

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.rules.TestWatcher
import org.junit.runner.Description

/**
 * JUnit 4 Rule that replaces [Dispatchers.Main] with a [TestDispatcher] for unit testing.
 *
 * Usage:
 * ```
 * @get:Rule val mainDispatcherRule = MainDispatcherRule()
 *
 * @Test
 * fun myTest() = runTest(mainDispatcherRule.testDispatcher) { ... }
 * ```
 *
 * Sharing the same [testDispatcher] between the Rule and [runTest] ensures that
 * [viewModelScope] coroutines and the test scope use the same [TestCoroutineScheduler],
 * allowing [advanceUntilIdle] / [runCurrent] to control ViewModel coroutines from tests.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class MainDispatcherRule(
    val testDispatcher: TestDispatcher = StandardTestDispatcher()
) : TestWatcher() {

    override fun starting(description: Description) {
        Dispatchers.setMain(testDispatcher)
    }

    override fun finished(description: Description) {
        Dispatchers.resetMain()
    }
}
