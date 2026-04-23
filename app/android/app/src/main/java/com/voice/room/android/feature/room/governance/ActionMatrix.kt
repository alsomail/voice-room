package com.voice.room.android.feature.room.governance

/**
 * 用户角色枚举（T-30040）
 *
 * 与服务端协议字符串对应：
 * - [Owner]  → "owner"
 * - [Admin]  → "admin"
 * - [Member] → "member"
 */
enum class Role {
    Owner, Admin, Member;

    companion object {
        /**
         * 将服务端返回的字符串角色转换为 [Role] 枚举。
         * 未知值默认返回 [Member]。
         */
        fun fromString(value: String): Role = when (value.lowercase()) {
            "owner" -> Owner
            "admin" -> Admin
            else -> Member
        }
    }
}

/**
 * 用户操作枚举（T-30040）
 *
 * 定义用户操作菜单 BottomSheet 中所有可能的操作项。
 * 实际可用项由 [computeActions] 根据权限矩阵计算。
 */
enum class UserAction {
    /** 任命目标用户为管理员（owner → member 时可用） */
    AssignAdmin,

    /** 卸任目标管理员（owner → admin 时可用） */
    RevokeAdmin,

    /** 强制目标上麦（目标不在麦时显示） */
    ForceTakeMic,

    /** 强制目标下麦（目标在麦时显示） */
    ForceLeaveMic,

    /** 禁麦（禁止目标在麦位发言） */
    MuteMic,

    /** 禁言（禁止目标发送文字消息） */
    MuteChat,

    /** 踢出房间（跳 T-30041 KickReasonDialog） */
    Kick,

    /** 查看资料（MVP 占位 Toast "即将上线"） */
    ViewProfile,

    /** 举报（MVP 占位 Toast） */
    Report,
}

/**
 * 权限矩阵纯函数（T-30040）
 *
 * 根据（我的角色, 目标角色, 目标是否在麦, 是否是自己）计算可用操作列表。
 *
 * ### 权限矩阵
 * | 我的角色 | 目标角色 | 可用操作 |
 * |--------|--------|--------|
 * | owner  | owner(自己) | 无（isSelf=true） |
 * | owner  | owner(他人) | ViewProfile + Report |
 * | owner  | admin  | RevokeAdmin + 抱上/下麦 + MuteMic + MuteChat + Kick + ViewProfile + Report |
 * | owner  | member | AssignAdmin + 抱上/下麦 + MuteMic + MuteChat + Kick + ViewProfile + Report |
 * | admin  | owner  | ViewProfile + Report |
 * | admin  | admin(他人) | ViewProfile + Report |
 * | admin  | member | 抱上/下麦 + MuteMic + MuteChat + Kick + ViewProfile + Report |
 * | member | 任何他人 | ViewProfile + Report |
 * | 任何   | self   | 空列表 |
 *
 * 注意：`抱上麦/抱下麦` 根据 `targetOnMic` 动态切换：
 * - 在麦（targetOnMic=true）→ 只显示 [UserAction.ForceLeaveMic]
 * - 不在麦（targetOnMic=false）→ 只显示 [UserAction.ForceTakeMic]
 *
 * @param myRole       当前登录用户的角色
 * @param targetRole   目标用户的角色
 * @param targetOnMic  目标用户是否在麦上
 * @param isSelf       目标是否为当前登录用户自己
 * @return 有序的可用操作列表（按菜单展示顺序）
 */
fun computeActions(
    myRole: Role,
    targetRole: Role,
    targetOnMic: Boolean,
    isSelf: Boolean,
): List<UserAction> {
    // 自己看自己：无操作
    if (isSelf) return emptyList()

    return when (myRole) {
        Role.Owner -> when (targetRole) {
            Role.Owner -> listOf(UserAction.ViewProfile, UserAction.Report)
            Role.Admin -> buildList {
                add(UserAction.RevokeAdmin)
                add(if (targetOnMic) UserAction.ForceLeaveMic else UserAction.ForceTakeMic)
                add(UserAction.MuteMic)
                add(UserAction.MuteChat)
                add(UserAction.Kick)
                add(UserAction.ViewProfile)
                add(UserAction.Report)
            }
            Role.Member -> buildList {
                add(UserAction.AssignAdmin)
                add(if (targetOnMic) UserAction.ForceLeaveMic else UserAction.ForceTakeMic)
                add(UserAction.MuteMic)
                add(UserAction.MuteChat)
                add(UserAction.Kick)
                add(UserAction.ViewProfile)
                add(UserAction.Report)
            }
        }
        Role.Admin -> when (targetRole) {
            Role.Owner -> listOf(UserAction.ViewProfile, UserAction.Report)
            Role.Admin -> listOf(UserAction.ViewProfile, UserAction.Report)
            Role.Member -> buildList {
                add(if (targetOnMic) UserAction.ForceLeaveMic else UserAction.ForceTakeMic)
                add(UserAction.MuteMic)
                add(UserAction.MuteChat)
                add(UserAction.Kick)
                add(UserAction.ViewProfile)
                add(UserAction.Report)
            }
        }
        Role.Member -> listOf(UserAction.ViewProfile, UserAction.Report)
    }
}
