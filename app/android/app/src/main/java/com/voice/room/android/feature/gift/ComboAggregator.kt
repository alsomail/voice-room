package com.voice.room.android.feature.gift

import java.util.UUID

/**
 * 连击聚合器 (T-30030)
 *
 * 同一礼物 + 同一接收者，在 [windowMs] 时间窗口内的多次点击会聚合为一个 [Combo]，
 * 共享同一个 [Combo.msgId]（UUID）和递增的 [Combo.count]。
 *
 * **MVP 策略**：调用方在用户点击"送出"时 `press()` 获取 combo，
 * 然后立即调用 `flush()` 重置，避免窗口期内旧 combo 被复用到下一次发送。
 *
 * **连击窗口 UI 反馈**：窗口内重复 press 累加 count，可用于 UI 展示"x10"等连击效果。
 *
 * @param windowMs     连击聚合时间窗口（毫秒），默认 3000ms
 * @param timeProvider 时间来源函数，默认 `System::currentTimeMillis`，
 *                     可注入 Fake 时钟供单元测试控制时间（S30-07/08/15）
 */
class ComboAggregator(
    private val windowMs: Long = 3000L,
    private val timeProvider: () -> Long = { System.currentTimeMillis() },
) {

    private var current: Combo? = null

    /**
     * 连击聚合数据（不可变）
     *
     * @param giftId      礼物 ID
     * @param recipientId 接收者 ID
     * @param msgId       本次连击的幂等 UUID（生命周期 = 一次聚合窗口）
     * @param count       当前累计数量
     * @param lastTs      最后一次 press 的时间戳（毫秒）
     */
    data class Combo(
        val giftId: String,
        val recipientId: String,
        val msgId: String,
        val count: Int,    // val：不可变，更新通过 copy() 实现
        val lastTs: Long,  // val：不可变，更新通过 copy() 实现
    )

    /**
     * 按下一次礼物按钮。
     *
     * - 若当前已有 combo 且 giftId/recipientId 一致，且距上次 press 未超出 [windowMs]：
     *   累加 [unitCount]，更新 [Combo.lastTs]，返回同一 combo。
     * - 否则（新礼物 / 新接收者 / 超时窗口）：创建新 combo，生成新 UUID。
     *
     * @param giftId      礼物 ID
     * @param recipientId 接收者 ID
     * @param unitCount   本次点击增量（默认 1）
     * @return 当前有效的 [Combo]
     */
    fun press(giftId: String, recipientId: String, unitCount: Int = 1): Combo {
        val now = timeProvider()
        val c = current
        return if (c != null
            && c.giftId == giftId
            && c.recipientId == recipientId
            && (now - c.lastTs) < windowMs
        ) {
            // 不可变更新：通过 copy() 生成新 Combo，保持 equals()/hashCode() 稳定
            c.copy(count = c.count + unitCount, lastTs = now).also { current = it }
        } else {
            Combo(
                giftId = giftId,
                recipientId = recipientId,
                msgId = UUID.randomUUID().toString(),
                count = unitCount,
                lastTs = now,
            ).also { current = it }
        }
    }

    /**
     * 清除当前 combo（发送后调用），使下一次 press 必然创建新 combo 和新 msg_id。
     */
    fun flush() {
        current = null
    }
}
