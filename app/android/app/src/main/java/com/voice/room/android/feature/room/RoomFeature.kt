package com.voice.room.android.feature.room

import com.voice.room.android.common.FeatureDescriptor

object RoomFeature {
    val descriptor = FeatureDescriptor(
        title = "Room Hall",
        description = "大厅页数据链路：GET /api/v1/rooms → DTO → RoomItem → HallViewModel → HallScreen (LazyVerticalGrid + RoomCard)。支持 Coil 异步加载房主头像、在线人数展示和点击进房回调。"
    )
}
