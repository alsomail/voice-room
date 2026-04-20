package com.voice.room.android.data.room

import com.voice.room.android.data.auth.ApiException
import com.voice.room.android.data.remote.api.RoomApiService
import com.voice.room.android.data.remote.model.ApiResponse
import com.voice.room.android.data.remote.model.RoomItemDto
import com.voice.room.android.data.remote.model.RoomListResponseData
import kotlinx.coroutines.test.runTest
import okhttp3.ResponseBody.Companion.toResponseBody
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import retrofit2.Response
import java.io.IOException

/**
 * TDD 单元测试 — RetrofitRoomRepository
 *
 * U01: HTTP 200 + code=0 + 1 item → Result.success(RoomsPage), 字段完整映射
 * U02: 网络 IOException → Result.failure(IOException)
 * U03: HTTP 500, error body {code:50001} → Result.failure(ApiException(50001))
 * U04: HTTP 200 + items=[], total=0 → Result.success(RoomsPage(total=0, items=emptyList()))
 * U05: owner_avatar=null → RoomItem.ownerAvatar == null, 不抛异常
 */
class RetrofitRoomRepositoryTest {

    // ─────────────────────────────────────────────
    // Fake API Service
    // ─────────────────────────────────────────────

    private class FakeRoomApiService(
        private val responseProvider: suspend () -> Response<ApiResponse<RoomListResponseData>>
    ) : RoomApiService {
        override suspend fun getRooms(page: Int, size: Int): Response<ApiResponse<RoomListResponseData>> =
            responseProvider()

        override suspend fun createRoom(
            request: com.voice.room.android.data.remote.model.CreateRoomRequest
        ): Response<ApiResponse<com.voice.room.android.data.remote.model.CreateRoomResponseData>> =
            throw UnsupportedOperationException("createRoom not tested in RetrofitRoomRepositoryTest")
    }

    // ─────────────────────────────────────────────
    // Test helpers
    // ─────────────────────────────────────────────

    private fun successResponse(data: RoomListResponseData): Response<ApiResponse<RoomListResponseData>> =
        Response.success(
            ApiResponse(code = 0, message = "ok", data = data, requestId = "req-001")
        )

    private fun errorResponse(
        httpCode: Int,
        errorJson: String = """{"code":$httpCode,"message":"error $httpCode","request_id":"req-err"}"""
    ): Response<ApiResponse<RoomListResponseData>> =
        Response.error(httpCode, errorJson.toResponseBody())

    private val sampleDto = RoomItemDto(
        roomId = "room-uuid-001",
        title = "测试房间A",
        roomType = "normal",
        memberCount = 5,
        maxMembers = 20,
        ownerId = "user-001",
        ownerNickname = "Alice",
        ownerAvatar = "https://cdn.example.com/a.jpg",
        createdAt = "2024-01-01T00:00:00Z"
    )

    // ─────────────────────────────────────────────
    // U01: HTTP 200 + code=0 + 1 item → Result.success, 字段完整映射
    // ─────────────────────────────────────────────

    @Test
    fun `U01 HTTP 200 code 0 one item returns success RoomsPage with all fields mapped`() = runTest {
        val data = RoomListResponseData(total = 1, page = 1, size = 20, items = listOf(sampleDto))
        val service = FakeRoomApiService { successResponse(data) }
        val repo = RetrofitRoomRepository(service)

        val result = repo.getRooms(page = 1, size = 20)

        assertTrue("Expected success", result.isSuccess)
        val page = result.getOrThrow()
        assertEquals(1, page.total)
        assertEquals(1, page.page)
        assertEquals(1, page.items.size)

        val item = page.items.first()
        assertEquals("room-uuid-001", item.roomId)
        assertEquals("测试房间A", item.title)
        assertEquals("normal", item.roomType)
        assertEquals(5, item.memberCount)
        assertEquals(20, item.maxMembers)
        assertEquals("Alice", item.ownerNickname)
        assertEquals("https://cdn.example.com/a.jpg", item.ownerAvatar)
        assertEquals("2024-01-01T00:00:00Z", item.createdAt)
    }

    // ─────────────────────────────────────────────
    // U02: 网络 IOException → Result.failure(IOException)
    // ─────────────────────────────────────────────

    @Test
    fun `U02 network IOException returns Result failure with IOException`() = runTest {
        val service = FakeRoomApiService { throw IOException("Network unreachable") }
        val repo = RetrofitRoomRepository(service)

        val result = repo.getRooms(page = 1, size = 20)

        assertTrue("Expected failure", result.isFailure)
        assertTrue(
            "Expected IOException, got ${result.exceptionOrNull()?.javaClass?.simpleName}",
            result.exceptionOrNull() is IOException
        )
    }

    // ─────────────────────────────────────────────
    // U03: HTTP 500, error body {code:50001} → ApiException(50001)
    // ─────────────────────────────────────────────

    @Test
    fun `U03 HTTP 500 with error body returns Result failure with ApiException code 50001`() = runTest {
        val errorJson = """{"code":50001,"message":"Internal Server Error","request_id":"req-500"}"""
        val service = FakeRoomApiService { errorResponse(500, errorJson) }
        val repo = RetrofitRoomRepository(service)

        val result = repo.getRooms(page = 1, size = 20)

        assertTrue("Expected failure", result.isFailure)
        val ex = result.exceptionOrNull()
        assertNotNull(ex)
        assertTrue("Expected ApiException, got ${ex?.javaClass?.simpleName}", ex is ApiException)
        assertEquals(50001, (ex as ApiException).code)
    }

    // ─────────────────────────────────────────────
    // U04: HTTP 200 + items=[], total=0 → RoomsPage(total=0, items=emptyList())
    // ─────────────────────────────────────────────

    @Test
    fun `U04 HTTP 200 with empty items list returns success RoomsPage with empty items`() = runTest {
        val data = RoomListResponseData(total = 0, page = 1, size = 20, items = emptyList())
        val service = FakeRoomApiService { successResponse(data) }
        val repo = RetrofitRoomRepository(service)

        val result = repo.getRooms(page = 1, size = 20)

        assertTrue("Expected success", result.isSuccess)
        val page = result.getOrThrow()
        assertEquals(0, page.total)
        assertTrue("Expected empty items", page.items.isEmpty())
    }

    // ─────────────────────────────────────────────
    // U05: owner_avatar=null → RoomItem.ownerAvatar == null, 不抛异常
    // ─────────────────────────────────────────────

    @Test
    fun `U05 owner_avatar null maps to RoomItem ownerAvatar null without exception`() = runTest {
        val dtoWithNullAvatar = sampleDto.copy(ownerAvatar = null)
        val data = RoomListResponseData(total = 1, page = 1, size = 20, items = listOf(dtoWithNullAvatar))
        val service = FakeRoomApiService { successResponse(data) }
        val repo = RetrofitRoomRepository(service)

        val result = repo.getRooms(page = 1, size = 20)

        assertTrue("Expected success", result.isSuccess)
        val item = result.getOrThrow().items.first()
        assertNull("ownerAvatar should be null", item.ownerAvatar)
    }

    // ─────────────────────────────────────────────
    // B03: HTTP 200 但 code=40001 → ApiException(40001)
    // ─────────────────────────────────────────────

    @Test
    fun `B03 HTTP 200 but business code 40001 returns Result failure with ApiException`() = runTest {
        val service = FakeRoomApiService {
            Response.success(
                ApiResponse(code = 40001, message = "Bad request", data = null, requestId = "req-b03")
            )
        }
        val repo = RetrofitRoomRepository(service)

        val result = repo.getRooms(page = 1, size = 20)

        assertTrue("Expected failure", result.isFailure)
        val ex = result.exceptionOrNull()
        assertTrue("Expected ApiException", ex is ApiException)
        assertEquals(40001, (ex as ApiException).code)
    }

    // ─────────────────────────────────────────────
    // Extra: HTTP 200 + response body null → Result.failure
    // ─────────────────────────────────────────────

    @Test
    fun `HTTP 200 with null response body returns Result failure`() = runTest {
        val service = FakeRoomApiService {
            @Suppress("UNCHECKED_CAST")
            Response.success(null as ApiResponse<RoomListResponseData>?)
        }
        val repo = RetrofitRoomRepository(service)

        val result = repo.getRooms(page = 1, size = 20)

        assertTrue("Expected failure for null body", result.isFailure)
        assertNotNull(result.exceptionOrNull())
    }
}
