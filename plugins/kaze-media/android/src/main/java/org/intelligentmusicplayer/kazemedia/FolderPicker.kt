package org.intelligentmusicplayer.kazemedia

import android.content.Intent
import android.net.Uri

/** Storage Access Framework tree picker with persisted read access. */
object FolderPicker {
    fun intent(): Intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).addFlags(
        Intent.FLAG_GRANT_READ_URI_PERMISSION or
            Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION or
            Intent.FLAG_GRANT_PREFIX_URI_PERMISSION,
    )

    /** Returns the tree URI after persisting read permission, or null when cancelled. */
    fun accept(context: android.content.Context, data: Intent?): Uri? {
        val uri = data?.data ?: return null
        context.contentResolver.takePersistableUriPermission(
            uri,
            Intent.FLAG_GRANT_READ_URI_PERMISSION,
        )
        return uri
    }
}
