package org.intelligentmusicplayer.kazemedia

import android.Manifest
import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.util.Log
import androidx.activity.result.ActivityResult
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.PermissionState
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
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

@InvokeArg
class InstallArgs {
    var path: String = ""
}

@InvokeArg
class ShareArgs {
    var title: String? = null
    var text: String = ""
}

/** Reading music in shared storage: READ_MEDIA_AUDIO from Android 13, READ_EXTERNAL_STORAGE before. */
private val AUDIO_ALIAS = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) "audio" else "storage"

@TauriPlugin(
    permissions = [
        Permission(strings = [Manifest.permission.READ_MEDIA_AUDIO], alias = "audio"),
        Permission(strings = [Manifest.permission.READ_EXTERNAL_STORAGE], alias = "storage"),
    ],
)
class KazeMediaPlugin(private val activity: Activity) : Plugin(activity) {
    private val secrets by lazy { SecretStore(activity.applicationContext) }

    override fun load(webView: android.webkit.WebView) {
        connectMedia()
    }

    override fun onResume(activity: AppCompatActivity) {
        connectMedia()
    }

    private fun connectMedia() {
        PlaybackService.onAppOpened()
        PlaybackService.commandSink = { name, pos ->
            trigger("mediaCommand", JSObject().put("command", name).put("positionMs", pos))
        }
        ApkInstaller.onFailure = { message -> trigger("updateFailed", JSObject().put("message", message)) }
    }

    override fun onDestroy(activity: AppCompatActivity) {
        if (activity.isFinishing && !activity.isChangingConfigurations) {
            PlaybackService.closeForAppExit(activity.applicationContext)
        }
    }

    @Command
    fun updatePlayback(invoke: Invoke) {
        if (!PlaybackService.acceptsUpdates()) {
            invoke.resolve()
            return
        }
        val args = invoke.parseArgs(PlaybackArgs::class.java)
        val state = MirrorState(
            args.playing, args.title, args.artist, args.album, args.durationMs, args.positionMs ?: 0,
        )
        PlaybackService.latest = state
        val running = PlaybackService.instance
        if (running != null && !running.isStopping) {
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

    /** Asks for music access when it isn't granted yet; resolves `granted`. */
    @Command
    fun requestAudioAccess(invoke: Invoke) {
        if (getPermissionState(AUDIO_ALIAS) == PermissionState.GRANTED) {
            invoke.resolve(JSObject().put("granted", true))
        } else {
            requestPermissionForAlias(AUDIO_ALIAS, invoke, "audioAccessResult")
        }
    }

    @PermissionCallback
    fun audioAccessResult(invoke: Invoke) {
        invoke.resolve(JSObject().put("granted", getPermissionState(AUDIO_ALIAS) == PermissionState.GRANTED))
    }

    /** Opens the Android share sheet with plain text. */
    @Command
    fun share(invoke: Invoke) {
        val args = invoke.parseArgs(ShareArgs::class.java)
        val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, args.text)
        args.title?.let { send.putExtra(Intent.EXTRA_SUBJECT, it) }
        activity.startActivity(Intent.createChooser(send, args.title))
        invoke.resolve()
    }

    /**
     * Installs the downloaded update in place. The first time, Android needs
     * "Install unknown apps" allowed for Kaze: its settings page opens, and
     * the install continues when the user comes back.
     */
    @Command
    fun installApk(invoke: Invoke) {
        val path = invoke.parseArgs(InstallArgs::class.java).path
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !activity.packageManager.canRequestPackageInstalls()) {
            val allow = Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${activity.packageName}"))
            startActivityForResult(invoke, allow, "installAllowed")
        } else {
            installNow(invoke, path)
        }
    }

    @ActivityCallback
    fun installAllowed(invoke: Invoke, result: ActivityResult) {
        if (activity.packageManager.canRequestPackageInstalls()) {
            installNow(invoke, invoke.parseArgs(InstallArgs::class.java).path)
        } else {
            invoke.reject("Allow Kaze to install apps to update it")
        }
    }

    /** Back from Settings: download and install regardless of whether access was granted. */
    @ActivityCallback
    fun overlayAnswered(invoke: Invoke, result: ActivityResult) {
        invoke.resolve()
    }

    /**
     * Offers optional restart access before the download. Declining or returning
     * from Settings lets the Rust updater proceed; the answer is remembered.
     */
    @Command
    fun prepareUpdate(invoke: Invoke) {
        activity.runOnUiThread {
            UpdateRestartPrompt.show(activity, { openOverlaySettings(invoke) }, { invoke.resolve() })
        }
    }

    private fun openOverlaySettings(invoke: Invoke) {
        val overlay = Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:${activity.packageName}"))
        try {
            startActivityForResult(invoke, overlay, "overlayAnswered")
        } catch (error: ActivityNotFoundException) {
            Log.w("KazeUpdate", "Restart permission settings unavailable", error)
            invoke.resolve()
        }
    }

    private fun installNow(invoke: Invoke, path: String) {
        try {
            ApkInstaller.install(activity.applicationContext, java.io.File(path))
            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject(e.message ?: "Could not install the update")
        }
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
