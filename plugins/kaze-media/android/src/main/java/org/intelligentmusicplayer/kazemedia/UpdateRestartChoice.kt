package org.intelligentmusicplayer.kazemedia

/** Records one answer before either opening Settings or continuing the update. */
class UpdateRestartChoice(
    private val rememberAnswer: () -> Unit,
    private val openSettings: () -> Unit,
    private val continueUpdate: () -> Unit,
) {
    private var answered = false

    fun answer(allow: Boolean) {
        if (answered) return
        answered = true
        rememberAnswer()
        if (allow) openSettings() else continueUpdate()
    }

    companion object {
        fun shouldAsk(granted: Boolean, asked: Boolean): Boolean = !granted && !asked
    }
}
