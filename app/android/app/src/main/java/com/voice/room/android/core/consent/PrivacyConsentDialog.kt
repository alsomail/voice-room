package com.voice.room.android.core.consent

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import com.voice.room.android.R
import com.voice.room.android.core.analytics.ConsentMode
import com.voice.room.android.util.UiText

/**
 * 隐私数据收集同意弹窗（T-30035）
 *
 * 在用户首次启动 App 后（Splash 成功 + 未设置同意状态）展示。
 * 提供两个选项：
 * - [ConsentMode.CrashOnly]：仅崩溃上报（合规豁免）
 * - [ConsentMode.All]：同意全量数据收集
 *
 * R1 批 2 修复（缺陷 5）：所有用户可见文案均通过 [stringResource] / [UiText] 解析，
 * 配套 `res/values/strings.xml`（英文默认）+ `res/values-ar/strings.xml`（真阿语翻译）。
 * 严禁在源码中再出现中/阿/英文字面量。
 *
 * testTag：
 * - `privacy_consent_dialog` — 整个弹窗根容器
 * - `btn_privacy_agree` — [同意全部] 按钮
 * - `btn_privacy_crash_only` — [仅 Crash] 按钮
 *
 * @param onAgreeAll    用户点击 [同意全部] 回调
 * @param onCrashOnly   用户点击 [仅 Crash] 回调
 * @param onDismiss     弹窗被关闭（不允许强制关闭，此为系统 back 兜底）
 */
@Composable
fun PrivacyConsentDialog(
    onAgreeAll: () -> Unit,
    onCrashOnly: () -> Unit,
    onDismiss: () -> Unit = {}
) {
    val title = UiText.of(R.string.privacy_consent_title).asString()
    val body = UiText.of(R.string.privacy_consent_body).asString()
    val agreeAllLabel = UiText.of(R.string.privacy_consent_btn_agree_all).asString()
    val crashOnlyLabel = UiText.of(R.string.privacy_consent_btn_crash_only).asString()

    Dialog(onDismissRequest = onDismiss) {
        Card(
            modifier = Modifier
                .fillMaxWidth()
                .testTag("privacy_consent_dialog"),
            elevation = CardDefaults.cardElevation(defaultElevation = 8.dp)
        ) {
            Column(
                modifier = Modifier.padding(24.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold
                )

                Spacer(modifier = Modifier.height(16.dp))

                Text(
                    text = body,
                    style = MaterialTheme.typography.bodyMedium
                )

                Spacer(modifier = Modifier.height(24.dp))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    OutlinedButton(
                        onClick = onCrashOnly,
                        modifier = Modifier
                            .weight(1f)
                            .testTag("btn_privacy_crash_only")
                    ) {
                        Text(crashOnlyLabel)
                    }

                    Button(
                        onClick = onAgreeAll,
                        modifier = Modifier
                            .weight(1f)
                            .testTag("btn_privacy_agree")
                    ) {
                        Text(agreeAllLabel)
                    }
                }
            }
        }
    }
}
