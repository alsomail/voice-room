package com.voice.room.android.feature.room

/**
 * 房间分类枚举 (T-30036)
 *
 * @param key      API 传递的分类 key
 * @param label    UI 展示标签（中文）
 */
enum class RoomCategory(val key: String, val label: String) {
    CHAT("chat", "闲聊"),
    EMOTION("emotion", "情感"),
    MUSIC("music", "音乐"),
    GAME("game", "游戏"),
    MATCHMAKING("matchmaking", "交友"),
    OTHER("other", "其他");

    companion object {
        /** 根据 key 查找枚举值，未匹配返回 [CHAT] */
        fun fromKey(key: String): RoomCategory =
            entries.firstOrNull { it.key == key } ?: CHAT
    }
}
