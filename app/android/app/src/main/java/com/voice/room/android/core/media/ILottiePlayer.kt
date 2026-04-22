package com.voice.room.android.core.media

/**
 * Lottie 动画播放器防腐层接口 (T-30031)
 *
 * 业务层通过此接口与 Lottie SDK 交互，禁止在业务层直接引用 Lottie 具体实现。
 *
 * 生产实现：`LottiePlayerAdapter`（依赖 Lottie 库）
 * 测试/占位实现：[NoOpLottiePlayer]
 *
 * ### 防腐层原则
 * - 接口定义在 `core/` 层
 * - 具体 SDK 实现放在同 `core/` 层的 Adapter 中
 * - `feature/` 业务层只依赖此接口，不 `import com.airbnb.lottie.*`
 */
interface ILottiePlayer {

    /**
     * 预加载 Lottie 动画 JSON 文件到内存缓存。
     *
     * @param url 动画 JSON 的 URL（CDN 地址或本地 file:// URI）
     * @return `true` 表示预加载成功，`false` 表示失败（调用方可 fallback）
     */
    suspend fun preload(url: String): Boolean

    /**
     * 检查指定 URL 的动画是否已在缓存中。
     *
     * @param url 动画 JSON 的 URL
     * @return `true` 表示缓存命中
     */
    fun isCached(url: String): Boolean
}
