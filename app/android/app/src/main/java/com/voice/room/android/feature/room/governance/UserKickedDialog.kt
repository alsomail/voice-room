package com.voice.room.android.feature.room.governance

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.voice.room.android.feature.room.KickedState

/**
 * 被踢出房间全屏提示弹窗（T-30042）
 *
 * - 全屏覆盖，不可通过点击外部或返回键关闭（[DialogProperties.dismissOnClickOutside]=false）
 * - 展示踢出原因和 cooldown 时长
 * - 点击"知道了"后回调 [onAcknowledge]
 *
 * testTag: `dialog_kicked`（整体容器）, `btn_kicked_ack`（确认按钮）
 *
 * @param state       被踢状态，包含 reason 和 cooldownSec
 * @param onAcknowledge 点击"知道了"回调
 */
@Composable
fun UserKickedDialog(
    state: KickedState,
    onAcknowledge: () -> Unit,
) {
    Dialog(
        onDismissRequest = { /* 不允许外部关闭 */ },
        properties = DialogProperties(
            dismissOnClickOutside = false,
            dismissOnBackPress = false,
            usePlatformDefaultWidth = false,
        )
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Color(0xCC000000))
                .testTag("dialog_kicked"),
            contentAlignment = Alignment.Center,
        ) {
            Column(
                modifier = Modifier
                    .padding(horizontal = 40.dp)
                    .background(
                        color = Color(0xFF1E1E2E),
                        shape = RoundedCornerShape(16.dp),
                    )
                    .padding(24.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
            ) {
                Text(
                    text = "你已被移出房间",
                    color = Color.White,
                    fontSize = 18.sp,
                    fontWeight = FontWeight.Bold,
                )

                Spacer(modifier = Modifier.height(12.dp))

                val reasonText = when (state.reason) {
                    "spam"       -> "发送垃圾内容"
                    "harassment" -> "骚扰他人"
                    "abuse"      -> "辱骂他人"
                    else         -> state.reason.ifBlank { "违规行为" }
                }
                val cooldownMin = state.cooldownSec / 60

                Text(
                    text = "原因：$reasonText\n${cooldownMin} 分钟后可再次进入",
                    color = Color(0xFFBBBBBB),
                    fontSize = 14.sp,
                    textAlign = TextAlign.Center,
                    lineHeight = 22.sp,
                )

                Spacer(modifier = Modifier.height(24.dp))

                Button(
                    onClick = onAcknowledge,
                    modifier = Modifier.testTag("btn_kicked_ack"),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = Color(0xFFD4AF37),
                        contentColor = Color.Black,
                    ),
                    shape = RoundedCornerShape(8.dp),
                ) {
                    Text(
                        text = "知道了",
                        fontSize = 16.sp,
                        fontWeight = FontWeight.SemiBold,
                    )
                }
            }
        }
    }
}
