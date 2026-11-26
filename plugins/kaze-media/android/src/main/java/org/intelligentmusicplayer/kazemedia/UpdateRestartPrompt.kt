package org.intelligentmusicplayer.kazemedia

import android.app.Activity
import android.app.AlertDialog

/** Optional restart access: ask once, before downloading, without blocking a declined update. */
object UpdateRestartPrompt {
    fun show(activity: Activity, openPermissionSettings: () -> Unit, continueUpdate: () -> Unit) {
        val context = activity.applicationContext
        if (!UpdateRelaunch.shouldAsk(context)) {
            continueUpdate()
            return
        }
        val choice = UpdateRestartChoice(
            rememberAnswer = { UpdateRelaunch.markAsked(context) },
            openSettings = openPermissionSettings,
            continueUpdate = continueUpdate,
        )
        AlertDialog.Builder(activity)
            .setTitle("Restart after updating?")
            .setMessage(
                "Allow Display over other apps so Kaze can restart automatically after the update. " +
                    "In Settings, select Kaze and allow the permission, then return here. " +
                    "If you continue without it, the update will still install and close the app. " +
                    "You can open Kaze yourself afterwards. We won’t ask again.",
            )
            .setPositiveButton("OK") { _, _ -> choice.answer(true) }
            .setNegativeButton("UPDATE WITHOUT RESTART") { _, _ -> choice.answer(false) }
            .setOnCancelListener { choice.answer(false) }
            .show()
    }
}
