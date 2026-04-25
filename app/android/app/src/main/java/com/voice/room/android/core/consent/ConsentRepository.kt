package com.voice.room.android.core.consent

import com.voice.room.android.core.analytics.AnalyticsPort
import com.voice.room.android.core.analytics.ConsentMode

/**
 * 隐私同意状态管理器（T-30035）
 *
 * 封装 [ConsentStore] 的读写，并在模式变更时通知 [AnalyticsPort]。
 * 初始状态由 [ConsentStore.load] 决定；未设置时默认 [ConsentMode.CrashOnly]（合规兜底）。
 *
 * @param store         持久化存储（DataStore 或内存 fake）
 * @param analyticsPort 变更后同步通知
 */
class ConsentRepository(
    private val store: ConsentStore,
    private var analyticsPort: AnalyticsPort? = null
) {
    @Volatile
    private var _mode: ConsentMode = ConsentMode.CrashOnly

    /** 当前生效的同意模式 */
    val mode: ConsentMode get() = _mode

    /** 是否已完成首次设置（弹窗逻辑用） */
    @Volatile
    var isSet: Boolean = false
        private set

    /**
     * 在 AppContainer 完成 [AnalyticsPort] 装配后回填依赖（T-30035 R1 批 2）。
     *
     * 解除 AppContainer 中 ConsentRepository ↔ CompositeAnalyticsPort 的循环依赖：
     * - CompositeAnalyticsPort 需要 EventReportClient（其又需要 ConsentRepository）
     * - ConsentRepository.saveConsent 需要回调 AnalyticsPort.setConsent
     *
     * 仅在装配阶段调用，不应在运行期切换。
     */
    fun attachAnalyticsPort(port: AnalyticsPort) {
        this.analyticsPort = port
    }

    /**
     * 从 DataStore 加载已保存的模式（应在 Splash 时调用）。
     * 加载后更新内存状态。
     */
    suspend fun load() {
        val saved = store.load()
        if (saved != null) {
            _mode = saved
            isSet = true
        }
    }

    /**
     * 用户在隐私弹窗中选择后调用。
     * 持久化到 DataStore，并同步通知 [AnalyticsPort]。
     *
     * @param mode 用户选择的同意模式
     */
    suspend fun saveConsent(mode: ConsentMode) {
        store.save(mode)
        _mode = mode
        isSet = true
        analyticsPort?.setConsent(mode)
    }
}
