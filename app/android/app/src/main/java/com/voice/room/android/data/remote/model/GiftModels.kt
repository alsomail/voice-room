package com.voice.room.android.data.remote.model

import com.google.gson.annotations.SerializedName

/**
 * 礼物列表 DTO (T-30028)
 *
 * 映射 `GET /api/v1/gifts/list` 响应中 `data.items` 数组的每个元素。
 */
data class GiftDto(
    @SerializedName("id")            val id: String,
    @SerializedName("code")          val code: String,
    @SerializedName("name")          val name: String,
    @SerializedName("icon_url")      val iconUrl: String,
    @SerializedName("price")         val price: Long,
    @SerializedName("sort_order")    val sortOrder: Int,
    @SerializedName("tier")          val tier: Int,
    @SerializedName("effect_level")  val effectLevel: Int? = null,
    @SerializedName("animation_url") val animationUrl: String? = null,
)

/**
 * 礼物列表 data 包装 (BUG-GIFT-JSON-PARSE Round 7)
 *
 * 服务端真实响应：
 * ```json
 * { "code": 0, "data": { "items": [ ... ] }, ... }
 * ```
 *
 * 历史上客户端误以为 `data` 直接是 `List<GiftDto>`，导致 Gson 反序列化抛出
 * `IllegalStateException: Expected BEGIN_ARRAY but was BEGIN_OBJECT`，礼物面板永远
 * 显示空态。新增此包装类后，`data` 解析为 `GiftListData { items }`，再 `.items.map`
 * 转换为领域对象。
 */
data class GiftListData(
    @SerializedName("items") val items: List<GiftDto>,
)
