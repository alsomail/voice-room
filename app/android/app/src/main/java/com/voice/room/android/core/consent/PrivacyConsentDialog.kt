package com.voice.room.android.core.consent

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import com.voice.room.android.core.analytics.ConsentMode

/**
 * 隐私数据收集同意弹窗（T-30035）
 *
 * 在用户首次启动 App 后（Splash 成功 + 未设置同意状态）展示。
 * 提供两个选项：
 * - [ConsentMode.CrashOnly]：仅崩溃上报（合规豁免）
 * - [ConsentMode.All]：同意全量数据收集
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
                // 标题
                Text(
                    text = "数据收集说明",
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "جمع البيانات",
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )

                Spacer(modifier = Modifier.height(16.dp))

                // 说明文本
                Text(
                    text = """
                        为了持续改善您的使用体验，我们会收集以下匿名行为数据：
                        
                        • 功能使用情况（不含手机号等个人身份信息）
                        • 应用崩溃报告（帮助我们快速修复问题）
                        
                        所有数据均经过脱敏处理，不会上传任何手机号或个人账户信息。
                        
                        لتحسين تجربتك، نجمع بيانات سلوكية مجهولة الهوية وتقارير الأعطال فقط.
                    """.trimIndent(),
                    style = MaterialTheme.typography.bodyMedium
                )

                Spacer(modifier = Modifier.height(24.dp))

                // 按钮行
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    // 仅 Crash 按钮
                    OutlinedButton(
                        onClick = onCrashOnly,
                        modifier = Modifier
                            .weight(1f)
                            .testTag("btn_privacy_crash_only")
                    ) {
                        Text("仅 Crash")
                    }

                    // 同意全部按钮
                    Button(
                        onClick = onAgreeAll,
                        modifier = Modifier
                            .weight(1f)
                            .testTag("btn_privacy_agree")
                    ) {
                        Text("同意全部")
                    }
                }
            }
        }
    }
}
