package com.voice.room.android.core.consent

import com.voice.room.android.core.analytics.ConsentMode
import java.io.File
import java.util.Properties

/**
 * 基于文件（Properties）的同意存储生产实现（T-30035 Review Round 1 修复）
 *
 * 冷重启后同意状态不丢失。使用 Java Properties 文件格式，避免 DataStore kapt 复杂性，
 * 同时在 JVM 单元测试中可直接使用（无需 Android Context 或 Robolectric）。
 *
 * ### 在 Android 生产代码中注入示例
 * ```kotlin
 * val consentFile = File(context.filesDir, "consent/consent.properties")
 * val store: ConsentStore = DataStoreConsentStore(consentFile)
 * ```
 *
 * @param propertiesFile 持久化文件路径（Android 生产环境使用 context.filesDir 下的路径）
 */
class DataStoreConsentStore(
    private val propertiesFile: File
) : ConsentStore {

    companion object {
        private const val KEY_CONSENT_MODE = "consent_mode"
        private const val VALUE_ALL = "ALL"
        private const val VALUE_CRASH_ONLY = "CRASH_ONLY"
        private const val VALUE_NONE = "NONE"
    }

    /**
     * 从文件加载已保存的同意模式。
     * 文件不存在或无有效值时返回 null。
     */
    override suspend fun load(): ConsentMode? {
        if (!propertiesFile.exists()) return null

        val props = Properties()
        return try {
            propertiesFile.inputStream().use { stream ->
                props.load(stream)
            }
            when (props.getProperty(KEY_CONSENT_MODE)) {
                VALUE_ALL -> ConsentMode.All
                VALUE_CRASH_ONLY -> ConsentMode.CrashOnly
                VALUE_NONE -> ConsentMode.None
                else -> null
            }
        } catch (e: Exception) {
            // 文件损坏或读取失败时安全降级为 null（上层默认 CrashOnly）
            null
        }
    }

    /**
     * 将同意模式持久化到文件。
     * 文件父目录不存在时自动创建。
     */
    override suspend fun save(mode: ConsentMode) {
        propertiesFile.parentFile?.mkdirs()

        val props = Properties()
        props.setProperty(KEY_CONSENT_MODE, mode.toStorageValue())

        propertiesFile.outputStream().use { stream ->
            props.store(stream, "VoiceRoom Analytics Consent - DO NOT EDIT MANUALLY")
        }
    }

    private fun ConsentMode.toStorageValue(): String = when (this) {
        ConsentMode.All -> VALUE_ALL
        ConsentMode.CrashOnly -> VALUE_CRASH_ONLY
        ConsentMode.None -> VALUE_NONE
    }
}
