package io.github.valnesfjord.tgwsproxyrs.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.valnesfjord.tgwsproxyrs.LogLine
import io.github.valnesfjord.tgwsproxyrs.ProxyViewModel
import io.github.valnesfjord.tgwsproxyrs.R

@Composable
fun ProxyScreen(viewModel: ProxyViewModel) {
    val args by viewModel.args.collectAsStateWithLifecycle()
    val running by viewModel.running.collectAsStateWithLifecycle()
    val tgLink by viewModel.tgLink.collectAsStateWithLifecycle()
    val error by viewModel.error.collectAsStateWithLifecycle()
    val logs = viewModel.logs
    val listState = rememberLazyListState()
    val following by remember {
        // Reflects the last layout pass, i.e. where the viewport was *before*
        // the line that triggers the effect below arrived — which is exactly
        // the question being asked: was the user reading the tail, or had they
        // scrolled back into the history that autoscrolling would yank away?
        derivedStateOf { !listState.canScrollForward }
    }

    // Keyed on the newest line's id rather than on `logs.size`: the buffer is
    // capped, and its collector appends and trims the oldest line back to back
    // with no suspension between them, so from the 501st line on the size is
    // permanently 500 and Compose never even observes the intermediate 501.
    LaunchedEffect(logs.lastOrNull()?.id) {
        if (following && logs.isNotEmpty()) {
            listState.scrollToItem(logs.lastIndex)
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(PainHub.Bg)
            .windowInsetsPadding(WindowInsets.systemBars)
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp),
    ) {
        Spacer(Modifier.height(20.dp))
        Text(
            text = stringResource(R.string.app_title),
            color = PainHub.Ink,
            fontSize = 24.sp,
            fontWeight = FontWeight.Bold,
        )
        Spacer(Modifier.height(2.dp))
        Text(
            text = stringResource(R.string.app_tagline),
            color = PainHub.Muted,
            fontSize = 13.sp,
        )

        Spacer(Modifier.height(20.dp))
        StatusCard(
            running = running,
            error = error,
            linkAvailable = tgLink != null,
            onToggle = { if (running) viewModel.stop() else viewModel.start() },
            onOpenLink = viewModel::openLink,
        )

        Spacer(Modifier.height(12.dp))
        ArgsCard(args = args, enabled = !running, onArgsChange = viewModel::updateArgs)

        Spacer(Modifier.height(12.dp))
        LogCard(logs = logs, listState = listState, modifier = Modifier.height(240.dp))

        Spacer(Modifier.height(16.dp))
        AuthorCardTile(
            role = stringResource(R.string.card_role),
            name = stringResource(R.string.card_name),
            onClick = viewModel::openCard,
        )

        Text(
            text = stringResource(R.string.footer),
            color = PainHub.Muted,
            fontSize = 11.sp,
            modifier = Modifier.padding(horizontal = 4.dp, vertical = 12.dp),
        )
    }
}

@Composable
private fun StatusCard(
    running: Boolean,
    error: String?,
    linkAvailable: Boolean,
    onToggle: () -> Unit,
    onOpenLink: () -> Unit,
) {
    SurfaceCard(modifier = Modifier.fillMaxWidth()) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            StatusDot(running)
            Spacer(Modifier.size(10.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = stringResource(
                        if (running) R.string.status_running else R.string.status_stopped,
                    ),
                    color = PainHub.Ink,
                    fontSize = 16.sp,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    text = stringResource(
                        if (running) R.string.status_hint_running else R.string.status_hint_stopped,
                    ),
                    color = PainHub.Muted,
                    fontSize = 11.sp,
                )
            }
        }

        Spacer(Modifier.height(14.dp))
        PrimaryButton(
            text = stringResource(if (running) R.string.stop else R.string.start),
            onClick = onToggle,
            modifier = Modifier.fillMaxWidth(),
        )

        if (linkAvailable) {
            Spacer(Modifier.height(8.dp))
            GhostButton(
                text = stringResource(R.string.open_telegram_link),
                onClick = onOpenLink,
                modifier = Modifier.fillMaxWidth(),
            )
        }

        if (error != null) {
            Spacer(Modifier.height(10.dp))
            Text(text = error, color = PainHub.Danger, fontSize = 12.sp)
        }
    }
}

@Composable
private fun ArgsCard(args: String, enabled: Boolean, onArgsChange: (String) -> Unit) {
    SurfaceCard(modifier = Modifier.fillMaxWidth()) {
        Caption(stringResource(R.string.section_args))
        Spacer(Modifier.height(8.dp))
        OutlinedTextField(
            value = args,
            onValueChange = onArgsChange,
            modifier = Modifier.fillMaxWidth(),
            textStyle = TextStyle(
                fontFamily = FontFamily.Monospace,
                fontSize = 11.sp,
                lineHeight = 16.sp,
                color = if (enabled) PainHub.Ink else PainHub.Muted,
            ),
            minLines = 2,
            maxLines = 5,
            enabled = enabled,
            shape = RoundedCornerShape(10.dp),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = PainHub.Ink,
                unfocusedBorderColor = PainHub.Line,
                disabledBorderColor = PainHub.Line,
                focusedContainerColor = PainHub.Bg,
                unfocusedContainerColor = PainHub.Bg,
                disabledContainerColor = PainHub.Bg,
                cursorColor = PainHub.Accent,
                focusedTextColor = PainHub.Ink,
                unfocusedTextColor = PainHub.Ink,
            ),
        )
        if (!enabled) {
            Spacer(Modifier.height(6.dp))
            Text(
                text = stringResource(R.string.args_locked),
                color = PainHub.Muted,
                fontSize = 11.sp,
            )
        }
    }
}

@Composable
private fun LogCard(
    logs: List<LogLine>,
    listState: LazyListState,
    modifier: Modifier = Modifier,
) {
    SurfaceCard(
        modifier = modifier.fillMaxWidth(),
        contentPadding = PaddingValues(14.dp),
    ) {
        Caption(stringResource(R.string.section_log))
        Spacer(Modifier.height(8.dp))
        SelectionContainer(modifier = Modifier.fillMaxSize()) {
            LazyColumn(state = listState, modifier = Modifier.fillMaxSize()) {
                items(logs, key = { it.id }) { line ->
                    Text(
                        text = line.text,
                        style = TextStyle(
                            fontFamily = FontFamily.Monospace,
                            fontSize = 10.sp,
                            lineHeight = 14.sp,
                        ),
                        color = PainHub.Ink.copy(alpha = 0.75f),
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 1.dp),
                    )
                }
            }
        }
    }
}
