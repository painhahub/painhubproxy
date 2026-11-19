package io.github.valnesfjord.tgwsproxyrs

import android.content.Context
import android.content.SharedPreferences
import androidx.core.content.edit
import java.security.SecureRandom

/**
 * The MTProto secret this install uses.
 *
 * The core generates a fresh random secret on every start when `--secret` is
 * absent, so the `tg://proxy` link Telegram already stored stops matching
 * after a restart and every session is drained as a bad handshake.  Pinning
 * one value per device fixes that, and keeping it out of the source means a
 * published APK does not carry the author's secret.
 *
 * The value lives in [ProxyService.PREFS] and survives updates, so the link
 * stays the same across restarts, reinstalls over the same data and reboots.
 */
object ProxySecret {
    private const val KEY = "device_secret"

    /** `--secret <hex>` or `--secret=<hex>`; the core wants 16 bytes in hex. */
    private val SECRET_ARG = Regex("""--secret(?:\s+|=)([0-9a-fA-F]{32})""")
    private val VALID = Regex("[0-9a-fA-F]{32}")
    private val SPACES = Regex("""\s{2,}""")

    /**
     * Returns [args] with exactly one `--secret` naming this device's secret.
     *
     * Any secret the caller wrote is dropped: the stored link and the running
     * proxy have to agree, and only one of them is under the user's control.
     */
    fun applyTo(context: Context, args: String): String {
        val secret = deviceSecret(context)
        val stripped = SPACES.replace(SECRET_ARG.replace(args, " "), " ").trim()
        return "$stripped --secret $secret"
    }

    /** The persisted secret, generating it on first use. */
    fun deviceSecret(context: Context): String {
        val prefs = prefs(context)
        prefs.getString(KEY, null)?.takeIf { VALID.matches(it) }?.let { return it }
        // An install upgraded from a build that passed a fixed --secret keeps
        // that value, so the link Telegram stored before the update still
        // works instead of forcing the user to add the proxy again.
        val adopted = SECRET_ARG.find(prefs.getString(ProxyService.PREF_ARGS, null) ?: "")
            ?.groupValues?.get(1)
        val secret = adopted ?: randomSecret()
        prefs.edit { putString(KEY, secret) }
        return secret
    }

    /**
     * Persists the secret while the previous arguments are still in prefs.
     *
     * Callers that rewrite [ProxyService.PREF_ARGS] must call this first: once
     * the old arguments are gone there is nothing left to adopt.
     */
    fun ensure(context: Context) {
        deviceSecret(context)
    }

    private fun randomSecret(): String {
        val bytes = ByteArray(16)
        SecureRandom().nextBytes(bytes)
        return bytes.joinToString("") { "%02x".format(it) }
    }

    private fun prefs(context: Context): SharedPreferences =
        context.getSharedPreferences(ProxyService.PREFS, Context.MODE_PRIVATE)
}
