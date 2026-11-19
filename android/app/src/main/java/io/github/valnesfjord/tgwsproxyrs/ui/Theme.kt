package io.github.valnesfjord.tgwsproxyrs.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

/**
 * Dark, low-contrast palette taken from the author's card: near-black paper
 * and white ink. The app stays dark in every system theme — the card has no
 * light variant either.
 */
object PainHub {
    val Bg = Color(0xFF08080A)
    val Paper = Color(0xFF121215)
    val Surface = Color(0xFF1A1A1F)
    val Ink = Color(0xFFF2F2F4)
    val Muted = Color(0xFF8E8E96)
    val Line = Color(0xFF26262C)
    val Accent = Color(0xFFFFFFFF)
    val OnAccent = Color(0xFF08080A)
    val Ok = Color(0xFF6FD08C)
    val Danger = Color(0xFFFF7A7A)
}

private val PainHubColorScheme = darkColorScheme(
    primary = PainHub.Accent,
    onPrimary = PainHub.OnAccent,
    background = PainHub.Bg,
    onBackground = PainHub.Ink,
    surface = PainHub.Paper,
    onSurface = PainHub.Ink,
    surfaceVariant = PainHub.Surface,
    onSurfaceVariant = PainHub.Muted,
    outline = PainHub.Line,
    outlineVariant = PainHub.Line,
    error = PainHub.Danger,
)

@Composable
fun TgWsTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = PainHubColorScheme, content = content)
}
