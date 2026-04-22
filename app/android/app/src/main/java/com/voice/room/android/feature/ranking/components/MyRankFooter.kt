package com.voice.room.android.feature.ranking.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.voice.room.android.core.theme.MenaColors
import com.voice.room.android.domain.ranking.MyRank

/**
 * 底部固定的"我的排名"栏 (T-30033)
 *
 * - 已上榜：显示"我的：{rank} / {score}💎"
 * - 未入榜：显示"未上榜，继续加油"
 *
 * testTag: `my_rank_footer`
 */
@Composable
fun MyRankFooter(
    myRank: MyRank?,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .background(MenaColors.Surface)
            .testTag("my_rank_footer")
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (myRank == null || myRank.rank == null) {
            Text(
                text = "未上榜，继续加油",
                color = MenaColors.OnBackgroundSecondary,
                fontSize = 14.sp,
            )
        } else {
            Text(
                text = "我的：",
                color = MenaColors.OnBackgroundSecondary,
                fontSize = 14.sp,
            )
            Text(
                text = "第 ${myRank.rank} 名",
                color = MenaColors.OnBackground,
                fontSize = 14.sp,
                fontWeight = FontWeight.Bold,
            )
            Spacer(modifier = Modifier.weight(1f))
            Text(
                text = "%,d 💎".format(myRank.score),
                color = MenaColors.Primary,
                fontSize = 14.sp,
                fontWeight = FontWeight.Bold,
            )
        }
    }
}
