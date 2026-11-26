package org.intelligentmusicplayer.lifecycleqa

import android.content.Intent
import android.content.ComponentName
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import org.intelligentmusicplayer.kazemedia.MirrorState
import org.intelligentmusicplayer.kazemedia.PlaybackService
import org.intelligentmusicplayer.kazemedia.KazeMediaPlugin

/** Disposable silent media facade. No Rust, user profile, network, or audio. */
class MainActivity : AppCompatActivity() {
    private val plugin by lazy { KazeMediaPlugin(this) }

    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        setContentView(TextView(this).apply { text = "Kaze lifecycle QA — silent fixture" })
        connect()
        act(intent)
    }

    override fun onResume() { super.onResume(); connect() }

    override fun onDestroy() { plugin.onDestroy(this); super.onDestroy() }

    private fun connect() {
        plugin.onResume(this)
        PlaybackService.commandSink = { name, _ ->
            Log.i("KazeLifecycleQA", "backend command: $name")
            if (name == "stop") Handler(Looper.getMainLooper()).post { update(MirrorState()) }
        }
    }

    override fun onNewIntent(intent: Intent) { super.onNewIntent(intent); act(intent) }

    private fun act(intent: Intent) {
        when (intent.getStringExtra("qaAction")) {
            "play" -> update(MirrorState(playing = true, title = "QA silent fixture", durationMs = 600000))
            "pause" -> update(PlaybackService.latest.copy(playing = false))
            "stop" -> update(MirrorState())
            "restart" -> {
                update(MirrorState())
                update(MirrorState(playing = true, title = "Restart fixture", durationMs = 600000))
            }
            "externalStop" -> externalStop()
            "dismiss" -> finishAndRemoveTask()
            "brokenSinkDismiss" -> {
                PlaybackService.commandSink = { _, _ -> error("fixture: closed bridge") }
                finishAndRemoveTask()
            }
            "close" -> {
                finish()
            }
            "dismissWithLateEvent" -> {
                Handler(Looper.getMainLooper()).postDelayed({
                    update(MirrorState(playing = true, title = "Late fixture event", durationMs = 600000))
                }, 300)
                finishAndRemoveTask()
            }
            "idleService" -> {
                PlaybackService.latest = MirrorState()
                startService(Intent(this, PlaybackService::class.java))
            }
        }
    }

    private fun externalStop() {
        val token = SessionToken(this, ComponentName(this, PlaybackService::class.java))
        val future = MediaController.Builder(this, token).buildAsync()
        future.addListener({
            val controller = future.get()
            controller.stop()
            controller.release()
        }, ContextCompat.getMainExecutor(this))
    }

    private fun update(state: MirrorState) {
        if (!PlaybackService.acceptsUpdates()) return
        PlaybackService.latest = state
        val service = PlaybackService.instance
        if (service != null && !service.isStopping) service.apply(state)
        else if (state.playing) ContextCompat.startForegroundService(this, Intent(this, PlaybackService::class.java))
    }
}
