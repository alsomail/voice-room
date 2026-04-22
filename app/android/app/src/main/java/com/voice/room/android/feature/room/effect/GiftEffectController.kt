package com.voice.room.android.feature.room.effect

import com.voice.room.android.core.media.ILottiePlayer
import com.voice.room.android.core.media.NoOpLottiePlayer
import com.voice.room.android.core.ws.event.GiftReceivedEvent
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * L3 全屏 Lottie 特效数据（T-30031）
 *
 * @param animationUrl Lottie JSON 的 URL；空字符串表示使用本地 fallback 动画
 * @param durationMs   动画总时长（毫秒），由 effectLevel 决定：level >= 5 → 8s，否则 5s
 */
data class FullscreenAnim(
    val animationUrl: String,
    val durationMs: Long,
)

/**
 * L1 弹幕消息 UI 模型（T-30031）
 *
 * 在聊天列表中以金色样式展示礼物弹幕。
 * 同 (senderUserId, giftId, receiverUserId) 组合的连击消息共用同一条弹幕，
 * [count] 会被累加。
 */
data class GiftMessageUi(
    /** 首次弹幕的 msgId（作为 LazyColumn key，保持稳定） */
    val msgId: String,
    val senderUserId: String,
    val senderNickname: String,
    val senderAvatar: String?,
    val receiverUserId: String,
    val receiverNickname: String,
    val giftId: String,
    val giftName: String,
    val giftIconUrl: String,
    /** 累计数量（连击时递增） */
    val count: Int,
    /** effectLevel >= 3 时为 true，弹幕文字粗体展示 */
    val isBold: Boolean,
)

/**
 * 礼物特效调度控制器（T-30031）
 *
 * 接收 [onGiftReceived] 调用，根据 effectLevel 分发三层特效：
 *
 * | 层级 | 触发条件          | 实现                                      | 时长     |
 * |------|-----------------|------------------------------------------|---------|
 * | L1   | 全部礼物          | [giftMessages] 列表追加 / 连击累加 count   | 永久驻列 |
 * | L2   | effectLevel >= 2 | [micGlowTargetUserId] 设置接收者，2s 自动清 | 2s      |
 * | L3   | effectLevel >= 4 | [fullscreenEffect] 顺序播放，最多队列 3 个  | 5s / 8s |
 *
 * 设计原则：
 * - [onGiftReceived] 是普通函数（非 suspend），可从任意上下文调用
 * - L1/L2 效果在 [onGiftReceived] 内同步更新，立即可被 UI 读取
 * - L3 通过内部 Channel 排队，由后台协程顺序播放（防腐层隔离 Lottie SDK）
 * - 通过构造参数注入 [scope]，方便单元测试使用 backgroundScope + TestCoroutineScheduler
 *
 * 生产集成：RoomViewModel 解析 GiftReceived WS 消息后调用 [onGiftReceived]
 *
 * @param scope        协程作用域（生产 = viewModelScope，测试 = backgroundScope）
 * @param lottiePlayer Lottie 播放器防腐层接口（生产 = LottiePlayerAdapter，测试/MVP = [NoOpLottiePlayer]）
 */
class GiftEffectController(
    private val scope: CoroutineScope,
    private val lottiePlayer: ILottiePlayer = NoOpLottiePlayer(),
) {
    // ─── L1: 弹幕消息列表 ─────────────────────────────────────────────────────

    private val _giftMessages = MutableStateFlow<List<GiftMessageUi>>(emptyList())

    /** L1 弹幕消息列表，永久驻列，连击时累加 count */
    val giftMessages: StateFlow<List<GiftMessageUi>> = _giftMessages.asStateFlow()

    // ─── L2: 麦位光圈 ──────────────────────────────────────────────────────────

    private val _micGlowTargetUserId = MutableStateFlow<String?>(null)

    /**
     * 当前需要显示金色光圈的麦位用户 ID（L2）。
     * null 表示无光圈；2s 后由控制器自动清除。
     */
    val micGlowTargetUserId: StateFlow<String?> = _micGlowTargetUserId.asStateFlow()

    // ─── L3: 全屏 Lottie ──────────────────────────────────────────────────────

    private val _fullscreenEffect = MutableStateFlow<FullscreenAnim?>(null)

    /**
     * 当前全屏特效数据（L3）。
     * null 表示无全屏特效；由 L3 队列处理器顺序推送，[skipFullscreen] 可提前置 null。
     */
    val fullscreenEffect: StateFlow<FullscreenAnim?> = _fullscreenEffect.asStateFlow()

    /**
     * L3 排队通道。
     * 容量为 3（MVP 上限）：第一个事件正在播放时，最多缓冲 3 个排队。
     * 超出时 [trySend] 失败静默丢弃。
     */
    private val l3Queue = Channel<GiftReceivedEvent>(capacity = 3)

    /**
     * 当前 L3 动画的延迟 Job。
     * [skipFullscreen] 取消此 Job 以提前结束动画，触发 L3 队列处理器继续处理下一个。
     */
    private var currentL3DelayJob: Job? = null

    // ─── 初始化：启动 L3 队列处理协程 ───────────────────────────────────────────

    init {
        // 顺序处理 L3 队列（同时只播放一个）
        scope.launch {
            for (evt in l3Queue) {
                playL3(evt)
            }
        }
    }

    // ─── 公开操作：接收礼物事件 ────────────────────────────────────────────────

    /**
     * 处理一个礼物接收事件，同步分发 L1/L2 特效，异步排队 L3 特效。
     *
     * 可从任意上下文调用（不要求在协程内）。
     * 由 RoomViewModel 解析 WS GiftReceived 消息后调用。
     */
    fun onGiftReceived(evt: GiftReceivedEvent) {
        // L1：所有礼物（包括补偿消息）均写入弹幕（同步）
        addOrUpdateGiftMessage(evt)

        // 补偿消息：仅 L1，跳过 L2/L3
        if (evt.isReplay) return

        // L2：effectLevel >= 2 → 接收者麦位光圈（2s 自动清除，同步设置值）
        if (evt.effectLevel >= 2) {
            triggerMicGlow(evt.receiverUserId)
        }

        // L3：effectLevel >= 4 → 全屏 Lottie（排队最多 3 个，超出静默丢弃）
        if (evt.effectLevel >= 4) {
            l3Queue.trySend(evt)
        }
    }

    // ─── 公开操作：跳过当前全屏动画 ────────────────────────────────────────────

    /**
     * 跳过当前正在播放的 L3 全屏动画，立即置 null，并触发队列中的下一个（如有）。
     */
    fun skipFullscreen() {
        currentL3DelayJob?.cancel()
        _fullscreenEffect.value = null
    }

    // ─── 内部：L1 弹幕追加/连击累加 ────────────────────────────────────────────

    private fun addOrUpdateGiftMessage(evt: GiftReceivedEvent) {
        val comboKey = buildComboKey(evt.senderUserId, evt.giftId, evt.receiverUserId)
        val current = _giftMessages.value
        val existingIdx = current.indexOfFirst {
            buildComboKey(it.senderUserId, it.giftId, it.receiverUserId) == comboKey
        }
        if (existingIdx >= 0) {
            val mutable = current.toMutableList()
            mutable[existingIdx] = mutable[existingIdx].copy(
                count = mutable[existingIdx].count + evt.count
            )
            _giftMessages.value = mutable
        } else {
            _giftMessages.value = current + GiftMessageUi(
                msgId = evt.msgId,
                senderUserId = evt.senderUserId,
                senderNickname = evt.senderNickname,
                senderAvatar = evt.senderAvatar,
                receiverUserId = evt.receiverUserId,
                receiverNickname = evt.receiverNickname,
                giftId = evt.giftId,
                giftName = evt.giftName,
                giftIconUrl = evt.giftIconUrl,
                count = evt.count,
                isBold = evt.effectLevel >= 3,
            )
        }
    }

    // ─── 内部：L2 麦位光圈触发 ─────────────────────────────────────────────────

    private fun triggerMicGlow(receiverUserId: String) {
        _micGlowTargetUserId.value = receiverUserId  // 同步设置
        scope.launch {
            delay(MIC_GLOW_DURATION_MS)
            if (_micGlowTargetUserId.value == receiverUserId) {
                _micGlowTargetUserId.value = null
            }
        }
    }

    // ─── 内部：L3 全屏动画播放 ─────────────────────────────────────────────────

    private suspend fun playL3(evt: GiftReceivedEvent) {
        val animUrl = evt.giftAnimationUrl ?: ""
        // MEDIUM-2: 通过防腐层接口预加载 Lottie 动画，失败时使用 fallback（不阻塞播放流程）
        if (animUrl.isNotEmpty()) {
            lottiePlayer.preload(animUrl) // false = fallback，UI 层展示本地 fallback
        }
        val durationMs = if (evt.effectLevel >= 5) L3_DURATION_LEVEL5_MS else L3_DURATION_DEFAULT_MS
        _fullscreenEffect.value = FullscreenAnim(
            animationUrl = evt.giftAnimationUrl ?: "",
            durationMs = durationMs,
        )
        currentL3DelayJob = scope.launch { delay(durationMs) }
        currentL3DelayJob!!.join()
        _fullscreenEffect.value = null
        currentL3DelayJob = null
    }

    private fun buildComboKey(senderUserId: String, giftId: String, receiverUserId: String): String =
        "$senderUserId|$giftId|$receiverUserId"

    companion object {
        internal const val MIC_GLOW_DURATION_MS = 2_000L
        internal const val L3_DURATION_DEFAULT_MS = 5_000L
        internal const val L3_DURATION_LEVEL5_MS = 8_000L
    }
}
