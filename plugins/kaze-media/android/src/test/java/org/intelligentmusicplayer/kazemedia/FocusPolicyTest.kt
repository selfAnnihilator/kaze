package org.intelligentmusicplayer.kazemedia

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class FocusPolicyTest {
    @Test
    fun transientLossPausesAndResumesOnGain() {
        val policy = FocusPolicy()
        assertTrue(policy.onTransientLoss(playing = true))
        policy.onState(playing = false, hasTrack = true)
        assertTrue(policy.shouldHoldFocus(playing = false))
        assertTrue(policy.onGain())
        assertFalse(policy.onGain())
    }

    @Test
    fun seekWhilePausedByInterruptionKeepsResume() {
        val policy = FocusPolicy()
        policy.onTransientLoss(playing = true)
        policy.onState(playing = false, hasTrack = true) // interruption pause
        policy.onState(playing = false, hasTrack = true) // seek snapshot
        assertTrue(policy.shouldHoldFocus(playing = false))
        assertTrue(policy.onGain())
    }

    @Test
    fun manualResumeDuringInterruptionCancelsPendingResume() {
        val policy = FocusPolicy()
        policy.onTransientLoss(playing = true)
        policy.onState(playing = false, hasTrack = true)
        policy.onState(playing = true, hasTrack = true)
        assertFalse(policy.onGain())
    }

    @Test
    fun stopDuringInterruptionReleasesFocus() {
        val policy = FocusPolicy()
        policy.onTransientLoss(playing = true)
        policy.onState(playing = false, hasTrack = false)
        assertFalse(policy.shouldHoldFocus(playing = false))
        assertFalse(policy.onGain())
    }

    @Test
    fun permanentLossAndManualResumeClearPending() {
        val policy = FocusPolicy()
        policy.onTransientLoss(playing = true)
        policy.onLoss()
        assertFalse(policy.onGain())
        policy.onTransientLoss(playing = true)
        policy.onState(playing = true, hasTrack = true)
        assertFalse(policy.resumeOnGain)
    }

    @Test
    fun lossWhilePausedDoesNotAskForPause() {
        assertFalse(FocusPolicy().onTransientLoss(playing = false))
    }
}
