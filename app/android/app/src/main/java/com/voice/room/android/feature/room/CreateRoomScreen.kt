package com.voice.room.android.feature.room

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Snackbar
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.voice.room.android.feature.room.create.components.AnnouncementField
import com.voice.room.android.feature.room.create.components.CategoryDropdown
import com.voice.room.android.feature.room.create.components.CoverPickerBottomSheet
import com.voice.room.android.feature.room.create.components.PasswordInputRow

/**
 * 创建房间页面（T-30036）
 *
 * 包含：
 * - 顶部 TopAppBar（返回按钮）
 * - 房名输入（最多 30 字符）
 * - 封面选择（点击触发内置 [CoverPickerBottomSheet]，R1 HIGH-02 修复）
 * - 分类下拉选择器 [CategoryDropdown]
 * - 公告输入 [AnnouncementField]（最多 200 字符）
 * - 密码开关 + 6 位分格输入 [PasswordInputRow]
 * - 提交按钮（`Key('btn_submit_create_room')`）
 * - 错误 Snackbar
 *
 * @param viewModel          [CreateRoomViewModel]
 * @param onNavigateUp       返回按钮回调
 * @param onNavigateToRoom   创建成功后导航到房间页
 * @param onSelectCover      可选：外部选图回调（提供时替代内置 BottomSheet）
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CreateRoomScreen(
    viewModel: CreateRoomViewModel,
    onNavigateUp: () -> Unit,
    onNavigateToRoom: (roomId: String) -> Unit,
    onSelectCover: (() -> Unit)? = null
) {
    val state by viewModel.formState.collectAsState()
    val snackbarHostState = remember { SnackbarHostState() }

    // HIGH-02：内置封面选择器显隐状态（R1 修复：CoverPickerBottomSheet 直接集成在此）
    var showCoverPicker by remember { mutableStateOf(false) }

    // 错误时弹 Snackbar（C36-08）
    LaunchedEffect(state.error) {
        state.error?.let { msg ->
            snackbarHostState.showSnackbar(msg)
        }
    }

    // 创建成功后导航（C36-07）
    LaunchedEffect(state.navigatedRoomId) {
        state.navigatedRoomId?.let { roomId ->
            viewModel.clearNavigation()
            onNavigateToRoom(roomId)
        }
    }

    // HIGH-02: 封面选择器 BottomSheet（内置，无需调用方注入）
    if (showCoverPicker) {
        CoverPickerBottomSheet(
            onCoverSelected = { url ->
                viewModel.updateCoverUrl(url)
                showCoverPicker = false
            },
            onDismiss = { showCoverPicker = false },
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("创建房间") },
                navigationIcon = {
                    IconButton(onClick = onNavigateUp) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "返回")
                    }
                }
            )
        },
        snackbarHost = {
            SnackbarHost(snackbarHostState) { data ->
                Snackbar(
                    snackbarData = data,
                    modifier = Modifier.testTag("create_room_snackbar")
                )
            }
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 24.dp, vertical = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {

            // ── 封面选择区域（HIGH-02 修复：点击触发内置 CoverPickerBottomSheet）──
            OutlinedCard(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(100.dp)
                    .testTag("cover_preview")
                    .clickable(
                        enabled = !state.submitting,
                        onClick = {
                            // 优先使用外部回调（测试/自定义场景），否则使用内置选择器
                            if (onSelectCover != null) onSelectCover() else showCoverPicker = true
                        }
                    ),
                shape = RoundedCornerShape(8.dp),
                border = BorderStroke(
                    width = 1.dp,
                    color = if (state.coverUrl.isEmpty())
                        MaterialTheme.colorScheme.error.copy(alpha = 0.5f)
                    else
                        MaterialTheme.colorScheme.primary
                )
            ) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    if (state.coverUrl.isEmpty()) {
                        Text(
                            text = "点击选择封面",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.testTag("cover_placeholder")
                        )
                    } else {
                        Text(
                            text = "✓ 封面已选择",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.testTag("cover_selected_hint")
                        )
                    }
                }
            }

            // ── 房间标题 ──────────────────────────────
            OutlinedTextField(
                value = state.title,
                onValueChange = viewModel::updateTitle,
                label = { Text("房间名称") },
                placeholder = { Text("请输入房间名称（最多 30 字符）") },
                supportingText = {
                    Text(
                        text = "${state.title.length}/30",
                        modifier = Modifier.testTag("title_counter")
                    )
                },
                isError = state.title.length > 30,
                singleLine = true,
                enabled = !state.submitting,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("input_room_title")
            )

            // ── 分类 ──────────────────────────────────
            CategoryDropdown(
                selected = state.category,
                onSelect = viewModel::updateCategory,
                enabled = !state.submitting
            )

            // ── 公告 ──────────────────────────────────
            AnnouncementField(
                value = state.announcement,
                onValueChange = viewModel::updateAnnouncement,
                enabled = !state.submitting
            )

            // ── 密码开关 ──────────────────────────────
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text(
                    text = "密码保护",
                    style = MaterialTheme.typography.bodyLarge
                )
                Switch(
                    checked = state.passwordEnabled,
                    onCheckedChange = viewModel::togglePasswordEnabled,
                    enabled = !state.submitting,
                    modifier = Modifier.testTag("switch_password")
                )
            }

            // ── 6 位密码输入（仅密码开关开启时显示）───────
            if (state.passwordEnabled) {
                PasswordInputRow(
                    value = state.password,
                    onValueChange = viewModel::updatePassword,
                    enabled = !state.submitting
                )
            }

            Spacer(Modifier.height(8.dp))

            // ── 提交按钮 ──────────────────────────────
            Button(
                onClick = viewModel::submit,
                enabled = state.canSubmit,
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("btn_submit_create_room")
            ) {
                if (state.submitting) {
                    CircularProgressIndicator(
                        modifier = Modifier.testTag("submit_loading"),
                        strokeWidth = 2.dp,
                        color = MaterialTheme.colorScheme.onPrimary
                    )
                } else {
                    Text("创建房间")
                }
            }
        }
    }
}
