package io.github.valnesfjord.tgwsproxyrs

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.SystemClock
import androidx.core.content.edit

/**
 * Remembers that the user wants the proxy running and keeps the watchdog armed.
 *
 * The flag is what separates "the user pressed Stop" from "the system killed
 * us": only the first one must stay down after a reboot or a watchdog tick.
 */
object ProxyAutostart {
    private const val KEY_WANTED = "want_running"

    /** Inexact interval: long enough to be cheap, short enough to recover. */
    private const val WATCHDOG_INTERVAL_MS = 15L * 60 * 1000
    private const val REQUEST_CODE = 4711

    fun prefs(context: Context) =
        context.getSharedPreferences(ProxyService.PREFS, Context.MODE_PRIVATE)

    fun setWanted(context: Context, wanted: Boolean) {
        prefs(context).edit { putBoolean(KEY_WANTED, wanted) }
    }

    fun isWanted(context: Context): Boolean = prefs(context).getBoolean(KEY_WANTED, false)

    private fun pendingIntent(context: Context): PendingIntent = PendingIntent.getBroadcast(
        context,
        REQUEST_CODE,
        Intent(context, WatchdogReceiver::class.java),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )

    /**
     * `setAndAllowWhileIdle` instead of an exact alarm: exact alarms need
     * SCHEDULE_EXACT_ALARM on Android 12+, and a 15-minute recovery window
     * does not justify asking the user for that permission.
     */
    fun scheduleWatchdog(context: Context) {
        val alarms = context.getSystemService(AlarmManager::class.java) ?: return
        alarms.setAndAllowWhileIdle(
            AlarmManager.ELAPSED_REALTIME_WAKEUP,
            SystemClock.elapsedRealtime() + WATCHDOG_INTERVAL_MS,
            pendingIntent(context),
        )
    }

    fun cancelWatchdog(context: Context) {
        context.getSystemService(AlarmManager::class.java)?.cancel(pendingIntent(context))
    }
}
