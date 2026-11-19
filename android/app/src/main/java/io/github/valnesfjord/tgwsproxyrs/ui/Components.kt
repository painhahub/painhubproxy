package io.github.valnesfjord.tgwsproxyrs.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** One solid colour plus a hairline border — no gradients, no elevation. */
val CardShape = RoundedCornerShape(14.dp)
private val ButtonShape = RoundedCornerShape(10.dp)

@Composable
fun SurfaceCard(
    modifier: Modifier = Modifier,
    contentPadding: PaddingValues = PaddingValues(16.dp),
    content: @Composable ColumnScope.() -> Unit,
) {
    Column(
        modifier = modifier
            .clip(CardShape)
            .background(PainHub.Paper)
            .border(1.dp, PainHub.Line, CardShape)
            .padding(contentPadding),
        content = content,
    )
}

/** Small muted caption above a block. */
@Composable
fun Caption(text: String, modifier: Modifier = Modifier) {
    Text(text = text, modifier = modifier, color = PainHub.Muted, fontSize = 12.sp)
}

/** The only high-contrast action on the screen. */
@Composable
fun PrimaryButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    Box(
        modifier = modifier
            .heightIn(min = 48.dp)
            .clip(ButtonShape)
            .background(if (enabled) PainHub.Accent else PainHub.Surface)
            .clickable(enabled = enabled, onClick = onClick)
            .padding(horizontal = 18.dp, vertical = 13.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            color = if (enabled) PainHub.OnAccent else PainHub.Muted,
            fontSize = 15.sp,
            fontWeight = FontWeight.SemiBold,
        )
    }
}

/** Secondary action: outlined, same shape as the primary one. */
@Composable
fun GhostButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .heightIn(min = 44.dp)
            .clip(ButtonShape)
            .border(1.dp, PainHub.Line, ButtonShape)
            .clickable(onClick = onClick)
            .padding(horizontal = 18.dp, vertical = 11.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(text = text, color = PainHub.Ink, fontSize = 14.sp, fontWeight = FontWeight.Medium)
    }
}

/** Green while the proxy runs, grey otherwise. */
@Composable
fun StatusDot(running: Boolean, modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .size(10.dp)
            .clip(CircleShape)
            .background(if (running) PainHub.Ok else PainHub.Line),
    )
}

/** The author's card: role on top, name below. Tapping opens the card page. */
@Composable
fun AuthorCardTile(
    role: String,
    name: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier
            .fillMaxWidth()
            .clip(CardShape)
            .background(PainHub.Paper)
            .border(1.dp, PainHub.Line, CardShape)
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 13.dp),
    ) {
        Text(text = role, color = PainHub.Muted, fontSize = 11.sp)
        Box(Modifier.height(3.dp))
        Text(
            text = name,
            color = PainHub.Ink,
            fontSize = 15.sp,
            fontWeight = FontWeight.SemiBold,
        )
    }
}
