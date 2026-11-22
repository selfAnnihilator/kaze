package org.intelligentmusicplayer.kazemedia

/**
 * Pure audio-focus decisions, kept free of Android types so they are unit
 * tested. Distinguishes a pause caused by a transient focus loss (resume when
 * focus returns) from an explicit user pause or stop (never auto-resume).
 */
class FocusPolicy {
    var resumeOnGain = false
        private set
    private var awaitingInterruptionPause = false

    /** True when the caller should ask Rust to pause. */
    fun onTransientLoss(playing: Boolean): Boolean {
        if (!playing) return false
        resumeOnGain = true
        awaitingInterruptionPause = true
        return true
    }

    /** Permanent loss: never resume automatically. */
    fun onLoss() {
        resumeOnGain = false
        awaitingInterruptionPause = false
    }

    /** True when the caller should ask Rust to resume. */
    fun onGain(): Boolean {
        val resume = resumeOnGain
        resumeOnGain = false
        awaitingInterruptionPause = false
        return resume
    }

    /** Folds a mirrored Rust state into the policy. */
    fun onState(playing: Boolean, hasTrack: Boolean) {
        val interruptionPause = awaitingInterruptionPause && !playing && hasTrack
        if (interruptionPause) {
            awaitingInterruptionPause = false
            return
        }
        // Playing again, explicit pause, or stop: any pending resume is void.
        resumeOnGain = false
        awaitingInterruptionPause = false
    }

    fun shouldHoldFocus(playing: Boolean): Boolean = playing || resumeOnGain
}
