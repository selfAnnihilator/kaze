package org.intelligentmusicplayer.kazemedia

import android.content.Context
import android.media.AudioManager
import android.content.Intent
import android.os.Looper
import android.os.PowerManager
import androidx.media.AudioAttributesCompat
import androidx.media.AudioFocusRequestCompat
import androidx.media.AudioManagerCompat
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture

/** Latest Rust playback state plus the sink for transport commands. */
data class MirrorState(
    val playing: Boolean = false,
    val title: String? = null,
    val artist: String? = null,
    val album: String? = null,
    val durationMs: Long? = null,
    val positionMs: Long = 0,
)

/**
 * Player facade that plays nothing. It mirrors Rust state and forwards user
 * transport actions (notification, lock screen, headset buttons) back to Rust.
 */
class ForwardingPlayer(looper: Looper, private val onCommand: (String, Long) -> Unit) :
    SimpleBasePlayer(looper) {
    private var mirror = MirrorState()

    fun update(state: MirrorState) {
        mirror = state
        invalidateState()
    }

    override fun getState(): State {
        val commands = Player.Commands.Builder().addAll(
            COMMAND_PLAY_PAUSE,
            COMMAND_SEEK_TO_NEXT,
            COMMAND_SEEK_TO_PREVIOUS,
            COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM,
            COMMAND_GET_CURRENT_MEDIA_ITEM,
            COMMAND_GET_METADATA,
        ).build()
        val builder = State.Builder().setAvailableCommands(commands)
        if (mirror.title == null) return builder.setPlaybackState(STATE_IDLE).build()
        val metadata = MediaMetadata.Builder()
            .setTitle(mirror.title).setArtist(mirror.artist).setAlbumTitle(mirror.album).build()
        val item = MediaItemData.Builder("kaze-current")
            .setMediaItem(MediaItem.Builder().setMediaMetadata(metadata).build())
            .setMediaMetadata(metadata)
            .apply { mirror.durationMs?.let { setDurationUs(it * 1000) } }
            .build()
        return builder
            .setPlaylist(listOf(item))
            .setCurrentMediaItemIndex(0)
            .setContentPositionMs(mirror.positionMs)
            .setPlayWhenReady(mirror.playing, PLAY_WHEN_READY_CHANGE_REASON_USER_REQUEST)
            .setPlaybackState(STATE_READY)
            .build()
    }

    override fun handleSetPlayWhenReady(playWhenReady: Boolean): ListenableFuture<*> {
        onCommand(if (playWhenReady) "play" else "pause", 0)
        return Futures.immediateVoidFuture()
    }

    override fun handleSeek(
        mediaItemIndex: Int,
        positionMs: Long,
        seekCommand: Int,
    ): ListenableFuture<*> {
        when (seekCommand) {
            COMMAND_SEEK_TO_NEXT, COMMAND_SEEK_TO_NEXT_MEDIA_ITEM -> onCommand("next", 0)
            COMMAND_SEEK_TO_PREVIOUS, COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> onCommand("previous", 0)
            else -> onCommand("seek", positionMs)
        }
        return Futures.immediateVoidFuture()
    }
}

/** Foreground `mediaPlayback` service keeping the process alive while Rust plays. */
class PlaybackService : MediaSessionService() {
    private var session: MediaSession? = null
    private var player: ForwardingPlayer? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private var focusRequest: AudioFocusRequestCompat? = null
    private val policy = FocusPolicy()

    private val focusListener = AudioManager.OnAudioFocusChangeListener { change ->
        when (change) {
            AudioManager.AUDIOFOCUS_LOSS -> {
                policy.onLoss()
                abandonFocus()
                commandSink?.invoke("pause", 0)
            }
            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT,
            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK ->
                if (policy.onTransientLoss(latest.playing)) commandSink?.invoke("pause", 0)
            AudioManager.AUDIOFOCUS_GAIN -> if (policy.onGain()) commandSink?.invoke("play", 0)
        }
    }

    override fun onCreate() {
        super.onCreate()
        val forwarding = ForwardingPlayer(Looper.getMainLooper()) { name, pos -> commandSink?.invoke(name, pos) }
        player = forwarding
        session = MediaSession.Builder(this, forwarding).build()
        instance = this
        forwarding.update(latest)
        syncWakeLock(latest.playing)
        syncFocus(latest.playing)
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? = session

    fun apply(state: MirrorState) {
        player?.update(state)
        syncWakeLock(state.playing)
        policy.onState(state.playing, state.title != null)
        syncFocus(state.playing)
    }

    private fun syncFocus(playing: Boolean) {
        if (!policy.shouldHoldFocus(playing)) {
            abandonFocus()
        } else if (playing && !requestFocus()) {
            // Rust already started playing; a refused request must stop it.
            commandSink?.invoke("pause", 0)
        }
    }

    /** True when focus is held or was just granted. */
    private fun requestFocus(): Boolean {
        if (focusRequest != null) return true
        val attributes = AudioAttributesCompat.Builder()
            .setUsage(AudioAttributesCompat.USAGE_MEDIA)
            .setContentType(AudioAttributesCompat.CONTENT_TYPE_MUSIC)
            .build()
        val request = AudioFocusRequestCompat.Builder(AudioManagerCompat.AUDIOFOCUS_GAIN)
            .setAudioAttributes(attributes)
            .setOnAudioFocusChangeListener(focusListener)
            .build()
        val manager = getSystemService(Context.AUDIO_SERVICE) as AudioManager
        val granted = AudioManagerCompat.requestAudioFocus(manager, request) ==
            AudioManager.AUDIOFOCUS_REQUEST_GRANTED
        if (granted) focusRequest = request
        return granted
    }

    private fun abandonFocus() {
        val request = focusRequest ?: return
        focusRequest = null
        val manager = getSystemService(Context.AUDIO_SERVICE) as AudioManager
        AudioManagerCompat.abandonAudioFocusRequest(manager, request)
    }

    private fun syncWakeLock(playing: Boolean) {
        val held = wakeLock?.isHeld == true
        if (playing && !held) {
            val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
            wakeLock = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "kaze:playback").apply { acquire() }
        } else if (!playing && held) {
            wakeLock?.release()
        }
    }

    override fun onTaskRemoved(rootIntent: Intent?) {
        if (player?.playWhenReady != true) stopSelf()
    }

    override fun onDestroy() {
        if (wakeLock?.isHeld == true) wakeLock?.release()
        abandonFocus()
        session?.release()
        player?.release()
        session = null
        player = null
        instance = null
        super.onDestroy()
    }

    companion object {
        @Volatile var latest = MirrorState()
        @Volatile var instance: PlaybackService? = null
        @Volatile var commandSink: ((String, Long) -> Unit)? = null
    }
}
