package com.voice.room.android.domain.gift

/**
 * 礼物值对象 (T-30028)
 *
 * @param id        服务端唯一 ID (UUID)
 * @param code      礼物编码（如 "castle_01"），用于动画资源路径拼接
 * @param name      展示名（已按 Accept-Language 本地化，后端返回）
 * @param iconUrl   礼物静态图标 URL
 * @param price     单价（钻石数）
 * @param sortOrder 排列顺序（越小越靠前，后端返回）
 * @param tier      礼物档次：1=普通，2=热门(Hot Tab)，3=精选(Hot Tab)
 */
data class GiftVO(
    val id: String,
    val code: String,
    val name: String,
    val iconUrl: String,
    val price: Long,
    val sortOrder: Int,
    val tier: Int,
)
