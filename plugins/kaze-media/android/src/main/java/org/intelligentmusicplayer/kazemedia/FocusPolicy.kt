package org.intelligentmusicplayer.kazemedia

/**
 * Pure audio-focus decisions, kept free of Android types so they are unit
 * tested. A pause caused by a transient focus loss resumes when focus returns;
 * a manual resume or a stop cancels that. Paused snapshots (seeks) keep it, so
 * an explicit pause while already interrupted cannot be told apart.
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

    /**
     * Folds a mirrored Rust state into the policy. Paused snapshots that
     * follow an interruption (seeks, position updates) keep the pending
     * resume; playing again or stopping voids it.
     */
    fun onState(playing: Boolean, hasTrack: Boolean) {
        if (playing || !hasTrack) {
            resumeOnGain = false
            awaitingInterruptionPause = false
        } else {
            awaitingInterruptionPause = false
        }
    }

    fun shouldHoldFocus(playing: Boolean): Boolean = playing || resumeOnGain
}
