package io.github.valnesfjord.tgwsproxyrs

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/**
 * Restores the proxy after a reboot, an app update or an OEM quick-boot.
 *
 * Same broadcast set the reference Android builds listen to; the arguments come
 * from the last run, so the proxy comes back with exactly the settings the user
 * left it with.
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val action = intent.action
        Log.i(TAG, "received $action")

        if (!ProxyAutostart.isWanted(context)) {
            Log.i(TAG, "autostart skipped: the user stopped the proxy")
            return
        }

        val args = ProxyAutostart.prefs(context).getString(ProxyService.PREF_ARGS, null)
            ?: ProxyService.DEFAULT_ARGS
        try {
            ProxyService.start(context, args)
            ProxyAutostart.scheduleWatchdog(context)
        } catch (e: Exception) {
            // Android 14+ may refuse a specialUse FGS started from a broadcast;
            // there is nothing to retry with here, so log it and let the
            // watchdog try again later.
            Log.w(TAG, "autostart refused by the system", e)
            ProxyAutostart.scheduleWatchdog(context)
        }
    }

    private companion object {
        const val TAG = "tg-ws-proxy-boot"
    }
}
