package org.intelligentmusicplayer.kazemedia

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class UpdateRestartChoiceTest {
    @Test
    fun onlyAnUnansweredMissingPermissionNeedsThePrompt() {
        assertTrue(UpdateRestartChoice.shouldAsk(granted = false, asked = false))
        assertFalse(UpdateRestartChoice.shouldAsk(granted = true, asked = false))
        assertFalse(UpdateRestartChoice.shouldAsk(granted = false, asked = true))
        assertFalse(UpdateRestartChoice.shouldAsk(granted = true, asked = true))
    }

    @Test
    fun approvingRemembersTheAnswerAndWaitsInSettings() {
        val events = mutableListOf<String>()
        val choice = choice(events)
        choice.answer(true)
        assertEquals(listOf("remember", "settings"), events)
        choice.answer(false)
        assertEquals(listOf("remember", "settings"), events)
    }

    @Test
    fun decliningOrDismissingContinuesWithoutOpeningSettingsOrAskingAgain() {
        val events = mutableListOf<String>()
        var asked = false
        val choice = UpdateRestartChoice(
            rememberAnswer = { asked = true; events.add("remember") },
            openSettings = { events.add("settings") },
            continueUpdate = { events.add("update") },
        )
        choice.answer(false)
        assertEquals(listOf("remember", "update"), events)
        assertFalse(UpdateRestartChoice.shouldAsk(granted = false, asked = asked))
        choice.answer(true)
        assertEquals(listOf("remember", "update"), events)
    }

    private fun choice(events: MutableList<String>) = UpdateRestartChoice(
        rememberAnswer = { events.add("remember") },
        openSettings = { events.add("settings") },
        continueUpdate = { events.add("update") },
    )
}
