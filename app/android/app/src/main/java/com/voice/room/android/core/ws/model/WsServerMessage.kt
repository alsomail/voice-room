package com.voice.room.android.core.ws.model

import com.google.gson.annotations.SerializedName

/**
 * S→C WebSocket 信令 sealed class（T-00101）
 *
 * 所有 payload 字段使用 @SerializedName("snake_case") + val camelCase。
 * 通过 [WsGsonFactory.create()] 创建的 Gson 实例进行反序列化。
 *
 * ## 协议规范
 * - payload-nested 信令（有 schema）：UserJoined / UserLeft / MicTaken / MicLeft /
 *   RoomMessage / UserMuted / Pong / 所有 Result 类型
 * - 平铺字段信令（无 schema 或向后兼容）：AdminChanged /
 *   RoomInfoUpdated / GiftReceived / UserKicked / MessageReceived / RoomClosed / Error
 * - 兜底：Unknown（type 未匹配任何已知信令）
 *
 * PROTO-BINDING: doc/protocol/schemas/ws/
 */
sealed class WsServerMessage {

    // ═══════════════════════════════════════════════════════════════════════════
    // S→C 广播信令（有协议 Schema，payload 嵌套）
    // ═══════════════════════════════════════════════════════════════════════════

    /**
     * 用户加入房间广播。
     * PROTO-BINDING: doc/protocol/schemas/ws/UserJoined.schema.json
     */
    data class UserJoined(
        val payload: UserJoinedPayload,
        @SerializedName("msg_id") val msgId: String? = null,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    data class UserJoinedPayload(
        @SerializedName("user_id") val userId: String,
        val nickname: String = "",
        val avatar: String? = null,
        @SerializedName("member_count") val memberCount: Int? = null,
        val role: String? = null,
    )

    /**
     * 用户离开房间广播。
     * PROTO-BINDING: doc/protocol/schemas/ws/UserLeft.schema.json
     */
    data class UserLeft(
        val payload: UserLeftPayload,
        @SerializedName("msg_id") val msgId: String? = null,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    data class UserLeftPayload(
        @SerializedName("user_id") val userId: String,
        val nickname: String? = null,
        @SerializedName("member_count") val memberCount: Int? = null,
    )

    /**
     * 麦位被占用广播。
     * PROTO-BINDING: doc/protocol/schemas/ws/MicTaken.schema.json
     */
    data class MicTaken(
        val payload: MicTakenPayload,
        @SerializedName("msg_id") val msgId: String? = null,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    data class MicTakenPayload(
        @SerializedName("mic_index") val micIndex: Int,
        @SerializedName("user_id") val userId: String,
        val nickname: String? = null,
        val avatar: String? = null,
        /**
         * 强制抱麦时由谁发起（服务端扩展字段，schema 未列出）
         * 用于 ForceTakeMic 流程中判断是否需要弹出权限请求。
         */
        @SerializedName("forced_by") val forcedBy: String? = null,
    )

    /**
     * 麦位释放广播。
     * PROTO-BINDING: doc/protocol/schemas/ws/MicLeft.schema.json
     */
    data class MicLeft(
        val payload: MicLeftPayload,
        @SerializedName("msg_id") val msgId: String? = null,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    data class MicLeftPayload(
        @SerializedName("mic_index") val micIndex: Int,
        @SerializedName("user_id") val userId: String? = null,
        /** 是否强制下麦（ForceLeaveMic 时为 true）*/
        val forced: Boolean? = null,
    )

    /**
     * 房间文本消息广播。
     * PROTO-BINDING: doc/protocol/schemas/ws/RoomMessage.schema.json
     */
    data class RoomMessage(
        val payload: RoomMessagePayload,
        @SerializedName("msg_id") val msgId: String? = null,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    data class RoomMessagePayload(
        @SerializedName("msg_id") val msgId: String,
        @SerializedName("user_id") val userId: String? = null,
        val nickname: String? = null,
        val avatar: String? = null,
        val content: String? = null,
    )

    /**
     * 服务端心跳应答。
     * PROTO-BINDING: doc/protocol/schemas/ws/Pong.schema.json
     */
    data class Pong(
        @SerializedName("msg_id") val msgId: String,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 加入房间结果。
     * PROTO-BINDING: doc/protocol/schemas/ws/JoinRoomResult.schema.json
     */
    data class JoinRoomResult(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val message: String? = null,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 离开房间结果。
     * PROTO-BINDING: doc/protocol/schemas/ws/LeaveRoomResult.schema.json
     */
    data class LeaveRoomResult(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 上麦结果。
     * PROTO-BINDING: doc/protocol/schemas/ws/TakeMicResult.schema.json
     */
    data class TakeMicResult(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val message: String? = null,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 下麦结果。
     * PROTO-BINDING: doc/protocol/schemas/ws/LeaveMicResult.schema.json
     */
    data class LeaveMicResult(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 发送消息结果。
     * PROTO-BINDING: doc/protocol/schemas/ws/SendMessageResult.schema.json
     */
    data class SendMessageResult(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val message: String? = null,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 发送礼物结果（S→C，仅发给发送方）。
     * PROTO-BINDING: doc/protocol/schemas/ws/SendGiftResult.schema.json
     */
    data class SendGiftResult(
        @SerializedName("msg_id") val msgId: String = "",
        val code: Int = 0,
        val message: String? = null,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 事件上报应答。
     * PROTO-BINDING: doc/protocol/schemas/ws/EventReportAck.schema.json
     */
    data class EventReportAck(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 踢人结果。
     * PROTO-BINDING: doc/protocol/schemas/ws/KickUserResult.schema.json
     */
    data class KickUserResult(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 禁言用户结果。
     * PROTO-BINDING: doc/protocol/schemas/ws/MuteUserResult.schema.json
     */
    data class MuteUserResult(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 解禁用户结果。
     * PROTO-BINDING: doc/protocol/schemas/ws/UnmuteUserResult.schema.json
     */
    data class UnmuteUserResult(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 转让管理员结果。
     * PROTO-BINDING: doc/protocol/schemas/ws/TransferAdminResult.schema.json
     */
    data class TransferAdminResult(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 强制上麦结果。
     * PROTO-BINDING: doc/protocol/schemas/ws/ForceTakeMicResult.schema.json
     */
    data class ForceTakeMicResult(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    /**
     * 强制下麦结果。
     * PROTO-BINDING: doc/protocol/schemas/ws/ForceLeaveMicResult.schema.json
     */
    data class ForceLeaveMicResult(
        @SerializedName("msg_id") val msgId: String? = null,
        val code: Int = 0,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    // ═══════════════════════════════════════════════════════════════════════════
    // S→C 信令（无 schema 或保持向后兼容的平铺字段格式）
    // ═══════════════════════════════════════════════════════════════════════════

    /**
     * 用户被禁麦/禁言广播（payload-nested，对齐 UserMuted.schema.json）。
     * PROTO-BINDING: doc/protocol/schemas/ws/UserMuted.schema.json
     */
    data class UserMuted(
        // PROTO-BINDING: doc/protocol/schemas/ws/UserMuted.schema.json
        val payload: UserMutedPayload,
        @SerializedName("msg_id") val msgId: String? = null,
        val timestamp: Long = 0,
    ) : WsServerMessage()

    data class UserMutedPayload(
        @SerializedName("room_id") val roomId: String? = null,
        @SerializedName("target_user_id") val targetUserId: String? = null,
        /** 禁用类型："mic" 或 "chat"（JSON 字段名 "type"） */
        @SerializedName("type") val muteType: String? = null,
        @SerializedName("duration_sec") val durationSec: Int = 0,
        @SerializedName("expires_at") val expiresAt: Long? = null,
        @SerializedName("operator_id") val operatorId: String? = null,
    )

    /**
     * 管理员变更广播（无独立 schema，平铺字段，向后兼容）。
     * PROTO-BINDING: No schema (backward-compat, flat fields)
     */
    data class AdminChanged(
        /** 被变更的目标用户 ID（camelCase，backward-compat） */
        @SerializedName("userId") val userId: String? = null,
        val role: String? = null,
        @SerializedName("msg_id") val msgId: String? = null,
    ) : WsServerMessage()

    /**
     * 房间信息更新广播（无 schema，平铺字段，向后兼容）。
     */
    data class RoomInfoUpdated(
        val title: String? = null,
        val announcement: String? = null,
        @SerializedName("msg_id") val msgId: String? = null,
    ) : WsServerMessage()

    /**
     * 礼物收到广播（无 schema，平铺字段，向后兼容）。
     * 字段匹配客户端既有解析代码（msgId camelCase, sender/receiver/gift 嵌套对象）。
     */
    data class GiftReceived(
        @SerializedName("msgId") val msgId: String? = null,
        @SerializedName("giftRecordId") val giftRecordId: String? = null,
        val sender: GiftUser? = null,
        val receiver: GiftUser? = null,
        val gift: GiftInfo? = null,
        val count: Int = 1,
        @SerializedName("totalPrice") val totalPrice: Long = 0L,
        @SerializedName("isReplay") val isReplay: Boolean = false,
    ) : WsServerMessage()

    data class GiftUser(
        @SerializedName("userId") val userId: String? = null,
        val nickname: String? = null,
        val avatar: String? = null,
    )

    data class GiftInfo(
        val id: String? = null,
        val code: String? = null,
        val name: String? = null,
        @SerializedName("icon_url") val iconUrl: String? = null,
        @SerializedName("animation_url") val animationUrl: String? = null,
        @SerializedName("effect_level") val effectLevel: Int = 1,
    )

    /**
     * 被踢出房间通知（无 schema，平铺字段，向后兼容）。
     */
    data class UserKicked(
        val reason: String? = null,
        @SerializedName("cooldown_sec") val cooldownSec: Int = 600,
        /** 兼容服务端将字段移入 payload 的新格式 */
        val payload: UserKickedPayload? = null,
        @SerializedName("msg_id") val msgId: String? = null,
    ) : WsServerMessage()

    data class UserKickedPayload(
        val reason: String? = null,
        @SerializedName("cooldown_sec") val cooldownSec: Int? = null,
    )

    /**
     * 旧版文本消息广播（无 schema，平铺字段，向后兼容）。
     * 服务端实际使用 RoomMessage，此类型保留以兼容测试和旧协议。
     */
    data class MessageReceived(
        @SerializedName("msgId") val msgId: String? = null,
        val senderNickname: String? = null,
        val content: String? = null,
        val timestamp: Long = 0L,
    ) : WsServerMessage()

    /**
     * 房间关闭通知（无 schema）。
     */
    data object RoomClosed : WsServerMessage()

    /**
     * 服务端错误通知（无 schema）。
     */
    data class ServerError(
        val code: Int? = null,
        val message: String? = null,
    ) : WsServerMessage()

    // ═══════════════════════════════════════════════════════════════════════════
    // 兜底：未知信令
    // ═══════════════════════════════════════════════════════════════════════════

    /**
     * 未匹配任何已知 type 的信令兜底类型。
     * handleWsMessage 中应记录日志并上报埋点，不抛异常。
     *
     * @param type 原始 type 字段值（"__missing_type__" 表示缺失 type 字段）
     */
    data class Unknown(val type: String) : WsServerMessage()
}
