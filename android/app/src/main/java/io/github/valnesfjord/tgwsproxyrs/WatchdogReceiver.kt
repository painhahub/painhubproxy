package io.github.valnesfjord.tgwsproxyrs

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/**
 * Periodic self-heal for ROMs that freeze or kill the foreground service.
 *
 * The alarm wakes the process even while the device is idle, which is also what
 * pulls it out of a vendor freezer long enough to check whether the native
 * proxy is still alive; if it is not, the service is started again.
 */
class WatchdogReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (!ProxyAutostart.isWanted(context)) return

        if (!NativeProxy.nativeIsRunning()) {
            Log.i(TAG, "proxy is down, restarting")
            val args = ProxyAutostart.prefs(context).getString(ProxyService.PREF_ARGS, null)
                ?: ProxyService.DEFAULT_ARGS
            try {
                ProxyService.start(context, args)
            } catch (e: Exception) {
                Log.w(TAG, "watchdog restart refused", e)
            }
        }

        ProxyAutostart.scheduleWatchdog(context)
    }

    private companion object {
        const val TAG = "tg-ws-proxy-watchdog"
    }
}
