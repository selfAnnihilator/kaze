package org.intelligentmusicplayer.kazemedia

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import android.provider.Settings
import android.util.Log

/**
 * Opens Kaze again after its in-app update replaced it.
 *
 * Android blocks an app from opening a screen on its own, and after an update
 * no part of Kaze is on screen. Apps allowed to "Display over other apps" are
 * exempt, so with that permission the new version starts itself; without it
 * the user reopens Kaze as before. Only updates started from inside Kaze
 * reopen it (a pending mark set just before installing), not installs from
 * a browser or a computer.
 */
object UpdateRelaunch {
    private const val PREFS = "kaze_update"
    private const val PENDING = "relaunch_pending"
    private const val ASKED = "overlay_asked"
    private const val TAG = "KazeUpdate"

    fun canRelaunch(context: Context): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.M || Settings.canDrawOverlays(context)

    /** The "Display over other apps" page is offered once; the user's answer stands after that. */
    fun shouldAsk(context: Context): Boolean =
        !canRelaunch(context) && !prefs(context).getBoolean(ASKED, false)

    fun markAsked(context: Context) = prefs(context).edit().putBoolean(ASKED, true).apply()

    /** Called just before the install is committed. */
    fun markPending(context: Context) = prefs(context).edit().putBoolean(PENDING, true).commit()

    private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    /** Runs in the new version once Android has replaced the app. */
    class Receiver : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            if (intent.action != Intent.ACTION_MY_PACKAGE_REPLACED) return
            val store = prefs(context)
            if (!store.getBoolean(PENDING, false)) return
            store.edit().remove(PENDING).apply()
            if (!canRelaunch(context)) {
                Log.i(TAG, "Updated; reopening needs \"Display over other apps\"")
                return
            }
            val launch = context.packageManager.getLaunchIntentForPackage(context.packageName) ?: return
            Log.i(TAG, "Updated; opening Kaze")
            context.startActivity(launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK))
        }
    }
}
