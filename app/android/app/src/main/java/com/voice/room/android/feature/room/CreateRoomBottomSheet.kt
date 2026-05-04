package com.voice.room.android.feature.room

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.SheetState
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
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
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp

/**
 * 创建房间 ModalBottomSheet (T-30007)
 *
 * 功能：
 * - 标题输入框（字符计数 helper text "x/30"）
 * - 房间类型 RadioButton（普通 / 密码 / 付费，水平排列）
 * - 密码输入框（仅 type=password 时可见）
 * - [创建] 按钮（Loading 时禁用 + CircularProgressIndicator）
 * - 创建成功后调用 [onSuccess] 回调并关闭 BottomSheet
 * - 创建失败后在按钮下方显示 Error 文本
 *
 * @param onSuccess    创建成功回调，参数为新房间 ID
 * @param onDismiss    关闭 BottomSheet 回调
 * @param viewModel    [CreateRoomViewModel]（默认使用 Factory 构建）
 * @param sheetState   BottomSheet 状态（用于测试注入）
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CreateRoomBottomSheet(
    onSuccess: (roomId: String) -> Unit,
    onDismiss: () -> Unit,
    viewModel: CreateRoomViewModel,
    sheetState: SheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
) {
    val uiState by viewModel.uiState.collectAsState()

    // BottomSheet 每次进入（Composition）时重置状态，防止上次 Success/Error 残留
    // 导致 LaunchedEffect(uiState) 在重新打开时触发旧回调（重复导航问题，HIGH-02 修复）
    LaunchedEffect(Unit) {
        viewModel.resetState()
    }

    // 成功后先 reset 再触发回调，确保再次打开 BottomSheet 时不会重放旧 Success
    // MEDIUM-03 注意：LaunchedEffect(uiState) 对"相同 Error 对象"不会重新触发（Compose 同值优化）。
    // 如需对连续相同错误重复弹提示，应将 Error 改为 Channel / SharedFlow one-shot 事件。
    LaunchedEffect(uiState) {
        if (uiState is CreateRoomUiState.Success) {
            viewModel.resetState()
            onSuccess((uiState as CreateRoomUiState.Success).roomId)
            onDismiss()
        }
    }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState
    ) {
        CreateRoomContent(
            uiState = uiState,
            onCreateClick = { title, type, password ->
                viewModel.createRoom(title, type, password)
            }
        )
    }
}

// ─────────────────────────────────────────────
// 内容区（可独立测试）
// ─────────────────────────────────────────────

private val roomTypes = listOf(
    "normal"   to "普通",
    "password" to "密码",
    "paid"     to "付费"
)

// MEDIUM-01 修复：不在 UI 层重复定义常量，直接引用 ViewModel 中的 internal const val
// 避免两处定义漂移（如 ViewModel 改了 30，UI 忘记同步）

/**
 * BottomSheet 内容区（解耦自 ModalBottomSheet，便于预览和测试）
 */
@Composable
internal fun CreateRoomContent(
    uiState: CreateRoomUiState,
    onCreateClick: (title: String, type: String, password: String?) -> Unit
) {
    var title by remember { mutableStateOf("") }
    var selectedType by remember { mutableStateOf("normal") }
    var password by remember { mutableStateOf("") }

    val isLoading = uiState is CreateRoomUiState.Loading
    val errorMessage = (uiState as? CreateRoomUiState.Error)?.message

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 24.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        // ── 标题 ─────────────────────────────────
        Text(
            text = "创建房间",
            style = MaterialTheme.typography.titleLarge,
            modifier = Modifier.testTag("create_room_title_label")
        )

        // ── 房间名称输入 ──────────────────────────
        OutlinedTextField(
            value = title,
            onValueChange = {
                // 允许最多输入 MAX_TITLE_LENGTH+1 个字符，多出的 1 个字符用于触发 isError 视觉反馈，
                // 实际超长校验由 ViewModel.createRoom() 兜底，+1 为纯 UX 提示意图（MEDIUM-02）
                if (it.codePointCount(0, it.length) <= CreateRoomViewModel.MAX_TITLE_LENGTH + 1) title = it
            },
            label = { Text("房间标题") },
            supportingText = {
                Text(
                    text = "${title.codePointCount(0, title.length)}/${CreateRoomViewModel.MAX_TITLE_LENGTH}",
                    modifier = Modifier.testTag("create_room_char_counter")
                )
            },
            singleLine = true,
            isError = title.codePointCount(0, title.length) > CreateRoomViewModel.MAX_TITLE_LENGTH,
            enabled = !isLoading,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("create_room_title_input")
        )

        // ── 房间类型 RadioButton ──────────────────
        Text(
            text = "房间类型",
            style = MaterialTheme.typography.labelLarge
        )
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .selectableGroup(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            roomTypes.forEach { (typeValue, typeLabel) ->
                Row(
                    modifier = Modifier
                        .selectable(
                            selected = selectedType == typeValue,
                            onClick = { selectedType = typeValue },
                            role = Role.RadioButton,
                            enabled = !isLoading
                        )
                        .testTag("create_room_type_$typeValue"),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    RadioButton(
                        selected = selectedType == typeValue,
                        onClick = null, // 由 selectable 处理
                        enabled = !isLoading
                    )
                    Text(
                        text = typeLabel,
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.padding(start = 4.dp)
                    )
                }
            }
        }

        // ── 密码输入框（仅 password 类型时显示）───────
        if (selectedType == "password") {
            OutlinedTextField(
                value = password,
                onValueChange = { password = it },
                label = { Text("房间密码") },
                singleLine = true,
                enabled = !isLoading,
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("create_room_password_input")
            )
        }

        // ── 错误提示 ──────────────────────────────
        if (errorMessage != null) {
            Text(
                // 缺陷 #4：UiText 通过 stringResource 渲染当前 locale 文案
                text = errorMessage.asString(),
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.testTag("create_room_error_text")
            )
        }

        Spacer(Modifier.height(4.dp))

        // ── [创建] 按钮 ────────────────────────────
        // BUG-GOVERNANCE-FORM-VALIDATE Round 6：必须在标题非空且长度合法时才允许提交，
        // 之前仅以 !isLoading 判断会让用户误点空标题进入服务端 400 路径。
        val trimmedTitleLength = title.trim().codePointCount(0, title.trim().length)
        val isTitleValid = title.isNotBlank() &&
            trimmedTitleLength in 1..CreateRoomViewModel.MAX_TITLE_LENGTH
        val isFormValid = isTitleValid &&
            (selectedType != "password" || password.isNotBlank())
        Button(
            onClick = {
                onCreateClick(
                    title,
                    selectedType,
                    if (selectedType == "password") password else null
                )
            },
            enabled = !isLoading && isFormValid,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("create_room_submit_button")
        ) {
            if (isLoading) {
                CircularProgressIndicator(
                    modifier = Modifier
                        .size(20.dp)
                        .testTag("create_room_loading_indicator"),
                    strokeWidth = 2.dp,
                    color = MaterialTheme.colorScheme.onPrimary
                )
            } else {
                Text("创建")
            }
        }

        Spacer(Modifier.height(16.dp))
    }
}
