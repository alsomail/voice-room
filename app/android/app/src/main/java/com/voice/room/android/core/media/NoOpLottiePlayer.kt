package com.voice.room.android.core.media

/**
 * [ILottiePlayer] 的 NoOp 实现（T-30031）
 *
 * MVP 占位实现：不依赖任何外部 Lottie SDK。
 * `preload()` 始终返回 `false`（UI 层展示本地 fallback 动画）。
 * `isCached()` 始终返回 `false`。
 *
 * 在引入真实 Lottie SDK 后，替换此实现为 `LottiePlayerAdapter`
 * 而无需改动任何业务层代码。
 */
class NoOpLottiePlayer : ILottiePlayer {
    override suspend fun preload(url: String): Boolean = false
    override fun isCached(url: String): Boolean = false
}
