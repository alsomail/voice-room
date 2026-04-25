package com.voice.room.android.core.storage

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import com.voice.room.android.core.analytics.queue.EventQueueEntity
import com.voice.room.android.core.analytics.queue.EventQueueRoomDao

/**
 * 应用全局 Room 数据库（T-30035 / R1 批 2 缺陷 7）
 *
 * 当前仅托管事件埋点队列表 `event_queue`；后续按需追加更多业务表。
 */
@Database(
    entities = [EventQueueEntity::class],
    version = 1,
    exportSchema = false
)
abstract class AppDatabase : RoomDatabase() {
    abstract fun eventQueueRoomDao(): EventQueueRoomDao

    companion object {
        private const val DB_NAME = "voice_room.db"

        @Volatile
        private var instance: AppDatabase? = null

        fun getInstance(context: Context): AppDatabase =
            instance ?: synchronized(this) {
                instance ?: Room.databaseBuilder(
                    context.applicationContext,
                    AppDatabase::class.java,
                    DB_NAME
                )
                    // 事件队列允许丢失（无外键依赖），开发期采用破坏性升级
                    .fallbackToDestructiveMigration()
                    .build()
                    .also { instance = it }
            }
    }
}
