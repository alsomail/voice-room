package com.voice.room.android.core.consent

import com.voice.room.android.core.analytics.ConsentMode
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

/**
 * DataStoreConsentStore TDD 测试 - Review Round 1 修复（T-30035）
 *
 * MEDIUM-2 验收：冷重启后同意状态不丢失（文件持久化）
 */
class DataStoreConsentStoreTest {

    @get:Rule
    val tempFolder = TemporaryFolder()

    // ── [RED] 冷重启持久化 ────────────────────────────────────────────────

    @Test
    fun `DataStoreConsentStore cold restart persists All mode`() = runTest {
        val file = tempFolder.newFile("consent.properties")

        // 模拟首次启动写入
        val store1 = DataStoreConsentStore(file)
        store1.save(ConsentMode.All)

        // 模拟冷重启：新建实例，从同一文件读取
        val store2 = DataStoreConsentStore(file)
        val loaded = store2.load()

        assertEquals("冷重启后应恢复 All 模式", ConsentMode.All, loaded)
    }

    @Test
    fun `DataStoreConsentStore cold restart persists CrashOnly mode`() = runTest {
        val file = tempFolder.newFile("consent_crash.properties")

        val store1 = DataStoreConsentStore(file)
        store1.save(ConsentMode.CrashOnly)

        val store2 = DataStoreConsentStore(file)
        assertEquals("冷重启后应恢复 CrashOnly 模式", ConsentMode.CrashOnly, store2.load())
    }

    @Test
    fun `DataStoreConsentStore cold restart persists None mode`() = runTest {
        val file = tempFolder.newFile("consent_none.properties")

        val store1 = DataStoreConsentStore(file)
        store1.save(ConsentMode.None)

        val store2 = DataStoreConsentStore(file)
        assertEquals("冷重启后应恢复 None 模式", ConsentMode.None, store2.load())
    }

    @Test
    fun `DataStoreConsentStore returns null when file is empty`() = runTest {
        val file = tempFolder.newFile("consent_empty.properties")
        // 文件存在但没有内容

        val store = DataStoreConsentStore(file)
        assertNull("未写入时 load 应返回 null", store.load())
    }

    @Test
    fun `DataStoreConsentStore returns null when file does not exist`() = runTest {
        val file = java.io.File(tempFolder.root, "nonexistent.properties")
        assertFalse("前提：文件不存在", file.exists())

        val store = DataStoreConsentStore(file)
        assertNull("文件不存在时 load 应返回 null", store.load())
    }

    @Test
    fun `DataStoreConsentStore overwrite updates persisted value`() = runTest {
        val file = tempFolder.newFile("consent_overwrite.properties")

        val store = DataStoreConsentStore(file)
        store.save(ConsentMode.CrashOnly)
        store.save(ConsentMode.All)  // 覆盖

        // 再新建实例验证
        val store2 = DataStoreConsentStore(file)
        assertEquals("最后一次保存应生效", ConsentMode.All, store2.load())
    }

    @Test
    fun `DataStoreConsentStore creates parent directories if needed`() = runTest {
        val nestedFile = java.io.File(tempFolder.root, "nested/dir/consent.properties")
        assertFalse("前提：目录不存在", nestedFile.parentFile.exists())

        val store = DataStoreConsentStore(nestedFile)
        store.save(ConsentMode.All)  // 应自动创建父目录

        assertTrue("保存后文件应存在", nestedFile.exists())
        val store2 = DataStoreConsentStore(nestedFile)
        assertEquals(ConsentMode.All, store2.load())
    }

    // ── DataStoreConsentStore 与 ConsentRepository 的集成 ────────────────

    @Test
    fun `ConsentRepository with DataStoreConsentStore persists across instances`() = runTest {
        val file = tempFolder.newFile("consent_repo.properties")
        val store1 = DataStoreConsentStore(file)

        // 第一个 repo 实例保存同意
        val repo1 = ConsentRepository(store1)
        repo1.saveConsent(ConsentMode.All)

        // 第二个 repo 实例（模拟冷重启）从持久化存储加载
        val store2 = DataStoreConsentStore(file)
        val repo2 = ConsentRepository(store2)
        repo2.load()

        assertEquals("冷重启后 repo 应恢复 All 模式", ConsentMode.All, repo2.mode)
        assertTrue("冷重启后 isSet 应为 true", repo2.isSet)
    }
}
