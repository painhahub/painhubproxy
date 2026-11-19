package io.github.valnesfjord.tgwsproxyrs

import android.app.Application
import android.content.ActivityNotFoundException
import android.content.Intent
import androidx.compose.runtime.mutableStateListOf
import androidx.core.content.edit
import androidx.core.net.toUri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class ProxyViewModel(application: Application) : AndroidViewModel(application) {
    private val prefs = application.getSharedPreferences(ProxyService.PREFS, 0)

    private val _args = MutableStateFlow(loadArgs())
    val args: StateFlow<String> = _args.asStateFlow()

    /**
     * Defaults ship with route fixes, but the copy stored in prefs would win
     * forever and keep the old ladder. Bumping [ARGS_VERSION] drops it once.
     */
    private fun loadArgs(): String {
        // Runs before the overwrite below: an upgrade adopts the secret from
        // the arguments it is about to replace, so the link Telegram already
        // stored keeps working instead of asking the user to add it again.
        ProxySecret.ensure(getApplication<Application>())
        val storedVersion = prefs.getInt(KEY_ARGS_VERSION, 0)
        val stored = prefs.getString(ProxyService.PREF_ARGS, null)
        if (storedVersion != ARGS_VERSION || stored.isNullOrBlank()) {
            prefs.edit {
                putInt(KEY_ARGS_VERSION, ARGS_VERSION)
                putString(ProxyService.PREF_ARGS, ProxyService.DEFAULT_ARGS)
            }
            return ProxyService.DEFAULT_ARGS
        }
        return stored
    }

    val running = ProxyBridge.running
    val tgLink = ProxyBridge.tgLink
    val error = ProxyBridge.error

    /**
     * Bounded log buffer the UI observes directly.  A `StateFlow<List<String>>`
     * copied the whole list to append and copied it again to trim, i.e. up to
     * a thousand element copies per line on a chatty proxy; a snapshot list
     * shares its backing structure between versions and lets Compose observe
     * the single append, so only the rows on screen recompose.
     */
    private val _logs = mutableStateListOf<LogLine>()
    val logs: List<LogLine> = _logs

    /** Never reused, never reordered — see [LogLine]. */
    private var nextLogId = 0L

    private var autoOpenPending = false

    init {
        ProxyBridge.syncFromNative()
        viewModelScope.launch {
            ProxyBridge.logs.collect { line ->
                _logs.add(LogLine(nextLogId++, line))
                if (_logs.size > MAX_LOG_LINES) {
                    _logs.removeAt(0)
                }
            }
        }
        viewModelScope.launch {
            ProxyBridge.tgLink.collect { link ->
                if (link != null && autoOpenPending) {
                    autoOpenPending = false
                    openLink()
                }
            }
        }
    }

    fun updateArgs(value: String) {
        _args.value = value
        prefs.edit { putString(ProxyService.PREF_ARGS, value) }
    }

    fun start() {
        ProxyBridge.clearError()
        autoOpenPending = true
        ProxyService.start(getApplication(), _args.value)
    }

    fun stop() {
        ProxyService.stop(getApplication())
    }

    fun openLink() {
        val link = tgLink.value ?: return
        val intent = Intent(Intent.ACTION_VIEW, link.toUri()).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        try {
            getApplication<Application>().startActivity(intent)
        } catch (_: ActivityNotFoundException) {
            ProxyBridge.reportMessage(getApplication<Application>().getString(R.string.telegram_missing))
        }
    }

    /** Opens the author's card — the same link the header icon and the bottom row use. */
    fun openCard() {
        val uri = AUTHOR_CARD_URL.toUri()
        val intent = Intent(Intent.ACTION_VIEW, uri).apply {
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        try {
            getApplication<Application>().startActivity(intent)
        } catch (_: ActivityNotFoundException) {
            ProxyBridge.reportMessage(getApplication<Application>().getString(R.string.browser_missing))
        }
    }

    companion object {
        /** Author's card — the single external link the UI exposes. */
        const val AUTHOR_CARD_URL = "https://painhahub.github.io/painhahubcard/"

        private const val MAX_LOG_LINES = 500

        /** Bump whenever [ProxyService.DEFAULT_ARGS] needs to replace stored args. */
        private const val ARGS_VERSION = 4
        private const val KEY_ARGS_VERSION = "args_version"
    }
}
