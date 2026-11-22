package org.intelligentmusicplayer.kazemedia

import android.app.Activity
import android.content.Intent
import androidx.activity.result.ActivityResult
import androidx.core.content.ContextCompat
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class PlaybackArgs {
    var playing: Boolean = false
    var title: String? = null
    var artist: String? = null
    var album: String? = null
    var durationMs: Long? = null
    var positionMs: Long? = null
}

@InvokeArg
class SecretArgs {
    var key: String = ""
    var value: String? = null
}

@TauriPlugin
class KazeMediaPlugin(private val activity: Activity) : Plugin(activity) {
    private val secrets by lazy { SecretStore(activity.applicationContext) }

    override fun load(webView: android.webkit.WebView) {
        PlaybackService.commandSink = { name, pos ->
            trigger("mediaCommand", JSObject().put("command", name).put("positionMs", pos))
        }
    }

    @Command
    fun updatePlayback(invoke: Invoke) {
        val args = invoke.parseArgs(PlaybackArgs::class.java)
        val state = MirrorState(
            args.playing, args.title, args.artist, args.album, args.durationMs, args.positionMs ?: 0,
        )
        PlaybackService.latest = state
        val running = PlaybackService.instance
        if (running != null) {
            running.apply(state)
        } else if (args.playing) {
            ContextCompat.startForegroundService(activity, Intent(activity, PlaybackService::class.java))
        }
        invoke.resolve()
    }

    @Command
    fun pickMusicFolder(invoke: Invoke) {
        startActivityForResult(invoke, FolderPicker.intent(), "folderPicked")
    }

    @ActivityCallback
    fun folderPicked(invoke: Invoke, result: ActivityResult) {
        val uri = if (result.resultCode == Activity.RESULT_OK) {
            FolderPicker.accept(activity, result.data)
        } else {
            null
        }
        invoke.resolve(JSObject().put("uri", uri?.toString()))
    }

    @Command
    fun secretSet(invoke: Invoke) {
        val args = invoke.parseArgs(SecretArgs::class.java)
        val value = args.value
        if (value == null) invoke.reject("missing value") else {
            secrets.set(args.key, value)
            invoke.resolve()
        }
    }

    @Command
    fun secretGet(invoke: Invoke) {
        val args = invoke.parseArgs(SecretArgs::class.java)
        invoke.resolve(JSObject().put("value", secrets.get(args.key)))
    }

    @Command
    fun secretDelete(invoke: Invoke) {
        secrets.delete(invoke.parseArgs(SecretArgs::class.java).key)
        invoke.resolve()
    }
}
