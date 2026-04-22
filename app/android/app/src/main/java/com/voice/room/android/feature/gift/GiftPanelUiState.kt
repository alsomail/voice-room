package com.voice.room.android.feature.gift

import com.voice.room.android.domain.gift.GiftVO
import com.voice.room.android.domain.gift.MicUserVO

/**
 * 礼物面板 Tab 枚举 (T-30028)
 *
 * - [Hot]      热门礼物，展示 tier∈[2,3] 的礼物
 * - [All]      全部礼物，展示后端返回的所有礼物
 * - [Backpack] Phase 2 占位，暂不实现
 */
enum class GiftTab { Hot, All, Backpack }

/**
 * 礼物面板 UI 状态 (T-30028)
 *
 * 全部字段不可变，由 ViewModel 通过 `copy()` 驱动更新。
 *
 * 计算属性：
 * - [selectedGift]          根据 [selectedGiftId] 从 [gifts] 中查找
 * - [totalPrice]            selectedGift.price × selectedCount
 * - [canSend]               礼物已选 + 接收者已选 + 余额充足 + 有人在麦
 * - [isBalanceInsufficient] 礼物已选且余额 < totalPrice
 * - [displayGifts]          按 [activeTab] 过滤后的展示列表
 */
data class GiftPanelUiState(
    /** 当前全量礼物列表（按 sort_order 排序，由后端保证） */
    val gifts: List<GiftVO> = emptyList(),

    /** 是否正在加载礼物列表 */
    val loading: Boolean = true,

    /** 错误描述（null = 无错误），非 null 时显示骨架屏 + "点击重试" */
    val error: String? = null,

    /** 当前选中礼物 ID（null = 未选中） */
    val selectedGiftId: String? = null,

    /** 当前选中数量档位，默认 1 */
    val selectedCount: Int = 1,

    /** 当前用户钻石余额（由 WS BalanceUpdated 实时更新） */
    val balance: Long = 0L,

    /** 当前在麦上的用户列表（从 RoomViewModel 传入） */
    val recipients: List<MicUserVO> = emptyList(),

    /** 当前选中接收者 ID（null = 未选中；默认主麦，由 T-30029 接入） */
    val selectedRecipientId: String? = null,

    /** 当前激活 Tab */
    val activeTab: GiftTab = GiftTab.Hot,

    /**
     * 是否正在发送礼物（等待 SendGiftResult 中）。
     *
     * - `true`：WS 已发出，等待服务端响应（按钮 disabled + 显示 CircularProgress）
     * - `false`：空闲状态（默认）
     *
     * [canSend] 包含 `!sending` 条件，确保发送期间按钮自动禁用（T-30030 S30-02）。
     */
    val sending: Boolean = false,
) {
    /** 根据 selectedGiftId 查找选中礼物 */
    val selectedGift: GiftVO? get() = gifts.firstOrNull { it.id == selectedGiftId }

    /** 总价 = 单价 × 数量 */
    val totalPrice: Long get() = (selectedGift?.price ?: 0L) * selectedCount

    /**
     * 是否可以送出：
     * - 有选中礼物
     * - 有选中接收者
     * - 余额 ≥ 总价
     * - 麦上有人（recipients 非空）
     * - **未正在发送中**（sending=false，防止重复发送，T-30030 S30-02）
     */
    val canSend: Boolean
        get() = selectedGift != null
            && selectedRecipientId != null
            && balance >= totalPrice
            && recipients.isNotEmpty()
            && !sending

    /**
     * 余额是否不足（用于 UI 显示"余额不足"文字）
     *
     * 条件：礼物已选 且 balance < totalPrice
     */
    val isBalanceInsufficient: Boolean
        get() = selectedGift != null && balance < totalPrice

    /**
     * 当前 Tab 下展示的礼物列表：
     * - [GiftTab.Hot]     → tier∈[2,3]
     * - [GiftTab.All]     → 全部
     * - [GiftTab.Backpack]→ 空（Phase 2 占位）
     */
    val displayGifts: List<GiftVO>
        get() = when (activeTab) {
            GiftTab.Hot      -> gifts.filter { it.tier in 2..3 }
            GiftTab.All      -> gifts
            GiftTab.Backpack -> emptyList()
        }
}
